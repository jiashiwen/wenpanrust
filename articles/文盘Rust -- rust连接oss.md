# 文盘Rust -- rust连接oss

对象存储是云的基础组件之一，各大云厂商都有相关产品。这里跟大家介绍一下rust与对象存储交到的基本套路和其中的一些技巧。

## 基本连接

我们以 [S3 sdk](https://github.com/awslabs/aws-sdk-rust)为例来说说基本的连接与操作，作者验证过aws、京东云、阿里云。主要的增删改查功能没有什么差别。

* 基本依赖
  Cargo.toml
  
  ``` toml
  # oss
  aws-config = { git = "https://github.com/awslabs/aws-sdk-rust", branch = "main" }
  aws-sdk-s3 = { git = "https://github.com/awslabs/aws-sdk-rust", branch = "main" }
  aws-types = { git = "https://github.com/awslabs/aws-sdk-rust", branch = "main",  feature = ["hardcoded-credentials"] }
  aws-credential-types = { git = "https://github.com/awslabs/aws-sdk-rust", branch = "main" }
  aws-smithy-types = { git = "https://github.com/awslabs/aws-sdk-rust", branch = "main" }
  ```

* 建立客户端

```rust
let shared_config = SdkConfig::builder()
         .credentials_provider(SharedCredentialsProvider::new(Credentials::new(
            "LTAI5t7NPuPKsXm6UeSa1",
            "DGHuK03ESXQYqQ83buKMHs9NAwz",
             None,
             None,
             "Static",
         )))
         .endpoint_url("http://oss-cn-beijing.aliyuncs.com")
         .region(Region::new("oss-cn-beijing"))
         .build();
let s3_config_builder = aws_sdk_s3::config::Builder::from(&shared_config);
let client = aws_sdk_s3::Client::from_conf(s3_config_builder.build());
```

建立Client所需要的参数主要有你需要访问的oss的AK、SK，endpoint url 以及服务所在的区域。以上信息都可以在服务商的帮助文档查询到。

* 对象列表

```rust
let mut obj_list = client
     .list_objects_v2()
     .bucket(bucket)
     .max_keys(max_keys)
     .prefix(prefix_str)
     .continuation_token(token_str);

let list = obj_list.send().await.unwrap();
println!("{:?}",list.contents());
println!("{:?}",list.next_continuation_token());
```

使用list_objects_v2函数返回对象列表，相比list_objects函数，list_objects_v2可以通过continuation_token和max_keys控制返回列表的长度。list.contents()返回对象列表数组，list.next_continuation_token()返回继续查询的token。

* 上传文件

```rust
let content = ByteStream::from("content in file".as_bytes());
 let exp = aws_smithy_types::DateTime::from_secs(100);
let upload = client
    .put_object()
    .bucket("bucket")
    .key("/test/key")
    .expires(exp)
    .body(content);
upload.send().await.unwrap();
```

指定bucket及对象路径，body接受ByteStream类型作为文件内容，最后设置过期时间expires，无过期时间时不指定该配置即可。

* 下载文件

```rust
let key = "/tmp/test/key".to_string();
let resp = client
    .get_object()
    .bucket("bucket")
    .key(&key)
    .send()
    .await.unwrap();
let data = resp.body.collect().await.unwrap();
let bytes = data.into_bytes();

let path = std::path::Path::new("/tmp/key")
if let Some(p) = path.parent() {
    std::fs::create_dir_all(p).unwrap();
}
let mut file = OpenOptions::new()
    .write(true)
    .truncate(true)
    .create(true)
    .open(path).unwrap();
let _ = file.write(&*bytes);
file.flush().unwrap();

```

通过get_object()函数获取GetObjectOutput。返回值的body 就是文件内容，将 body 转换为 bytes，最后打开文件写入即可。

* 删除文件

```rust
let mut keys = vec![];
let key1 = ObjectIdentifier::builder()
    .set_key(Some("/tmp/key1".to_string()))
    .build();
let key2 = ObjectIdentifier::builder()
    .set_key(Some("/tmp/key2".to_string()))
    .build()
keys.push(key1);
keys.push(key2)
client
    .delete_objects()
    .bucket(bucket)
    .delete(Delete::builder().set_objects(Some(keys)).build())
    .send()
    .await
    .unwrap();
```

delete_objects 批量删除对象。首先构建keys vector，定义要删除的对象，然后通过Delete::builder()，构建 Delete model。

## 大文件上传

```rust
let mut file = fs::File::open("/tmp/file_name").unwrap();
let chunk_size = 1024*1024;
let mut part_number = 0;
let mut upload_parts: Vec<CompletedPart> = Vec::new();

//获取上传id
let multipart_upload_res: CreateMultipartUploadOutput = self
    .client
    .create_multipart_upload()
    .bucket("bucket")
    .key("/tmp/key")
    .send()
    .await.unwrap();
let upload_id = match multipart_upload_res.upload_id() {
    Some(id) => id,
    None => {
        return Err(anyhow!("upload id is None"));
    }
};

//分段上传文件并记录completer_part
loop {
    let mut buf = vec![0; chuck_size];
    let read_count = file.read(&mut buf)?;
    part_number += 1;

    if read_count == 0 {
        break;
    }

    let body = &buf[..read_count];
    let stream = ByteStream::from(body.to_vec());

    let upload_part_res = self
        .client
        .upload_part()
        .key(key)
        .bucket(bucket)
        .upload_id(upload_id)
        .body(stream)
        .part_number(part_number)
        .send()
        .await.unwrap();

    let completer_part = CompletedPart::builder()
        .e_tag(upload_part_res.e_tag.unwrap_or_default())
        .part_number(part_number)
        .build();

    upload_parts.push(completer_part);

    if read_count != chuck_size {
        break;
    }
}
// 完成上传文件合并
let completed_multipart_upload: CompletedMultipartUpload =
    CompletedMultipartUpload::builder()
        .set_parts(Some(upload_parts))
        .build();

let _complete_multipart_upload_res = self
    .client
    .complete_multipart_upload()
    .bucket("bucket")
    .key(key)
    .multipart_upload(completed_multipart_upload)
    .upload_id(upload_id)
    .send()
    .await.unwrap();
```

有时候面对大文件，比如几百兆甚至几个G的文件，为了节约带宽和内存，我才采取分段上传的方案，然后在对象存储的服务端做合并。基本流程是：指定bucket和key，获取一个上传id；按流读取文件，分段上传字节流，并记录CompletedPart;通知服务器按照CompletedPart 集合来合并文件。具体过程代码已加注释，这里不再累述。

## 大文件下载

```rust
let mut file = match OpenOptions::new()
            .truncate(true)
            .create(true)
            .write(true)
            .open("/tmp/target_file");
let key = "/tmp/test/key".to_string();
let resp = client
    .get_object()
    .bucket("bucket")
    .key(&key)
    .send()
    .await.unwrap();

let content_len = resp.content_length();
let mut byte_stream_async_reader = resp.body.into_async_read();
let mut content_len_usize: usize = content_len.try_into().unwrap();
loop {
    if content_len_usize > chunk_size {
        let mut buffer = vec![0; chunk_size];
        let _ = byte_stream_async_reader.read_exact(&mut buffer).await.unwrap();
        file.write_all(&buffer).unwrap();
        content_len_usize -= chunk_size;
        continue;
    } else {
        let mut buffer = vec![0; content_len_usize];
        let _ = byte_stream_async_reader.read_exact(&mut buffer).await.unwrap();
        file.write_all(&buffer).unwrap();
        break;
    }
}
file.flush().unwrap();
```

在从对象存储服务端下载文件的过程中也会遇到大文件问题。为了节约带宽和内存，我们采取读取字节流的方式分段写入文件。首先get_object()函数获取ByteStream，通过async_reader流式读取对象字节，分段写入文件。

对象存储的相关话题今天先聊到这儿，下期见。