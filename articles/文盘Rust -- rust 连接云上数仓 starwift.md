# 文盘Rust -- rust 连接云上数仓 starwift

最近想看看 rust 如何集成 clickhouse，又犯了好吃懒做的心理（不想自己建环境），刚好京东云发布了兼容ck 的云原生数仓 Starwfit，于是搞了个实例折腾一番。
Starwfit 是京东云自主研发的新一代云原生数据仓库,通过存算分离降低了存储成本，同时兼具性能和扩展弹性。其写入和查询速度可达到传统数据仓库的数倍，为用户提供实时数据分析能力。广泛应用于流量分析、精准营销、用户画像、广告实时竞价、BI报表分析、日志分析、促销选品、物联网等业务场景。
广告打到这里，言归正传，看看 rust 如何与 starwift 打交道。

## 创建集群

* 登录[控制台](https://console.jdcloud.com/)，直接选配创建即可

  ![create instance](images/starwift_create.png)

* 创建用户
  ![create user](images/starwift_create_user.png)

* 开启白名单
  为了保证starwift能够被外部资源访问需要开启白名单，默认情况下新创建的实例只有该实例所在vpc下才能访问
  ![white list](images/starwift_white_list.png)
  为了方便，实验环境直接编辑 default 分组，编辑 ip 列表为 0.0.0.0/0，对所有ip开放。在实际生产环境需要根据实际情况配置白名单避免安全问题。
  
* 开启公网连接
  
  为了实验方便，开启公网访问域名便于开发和测试
  ![internet](images/starwift_internet.png)
  
## mac客户端安装

* 安装客户端

  macos 客户端安装
  
  ```shell
   curl -O 'https://builds.clickhouse.com/master/macos/clickhouse' && chmod a  +x ./clickhouse
  ```
  
  其他客户端安装 参见 [clickhosue 官方文档](https://clickhouse.com/docs/zh/)

* 测试连通性

  ```shell
  clickhouse client --host service-terrabase-9s29mdlsb7.terrabase-9s29mdlsb7-hb-public.jvessel2.jdcloud.com \
  --port 9000 \
  --user sample \
  --password xxxxxx
  ```
  
  ```shell
  clickhouse:) show databases
  ```

## rust 连接 starwift

   starwift 支持 tcp 和 http 两种协议连接server。下面我们分别给出两种方式的示例代码

* tcp 连接

  * 依赖 crate
  
    ```toml
    # clickhouse tcp
    tokio = { version = "1.21.2", features = ["full"] }
    clickhouse-rs = { git = "https://github.com/suharev7/clickhouse-rs", features = ["default"]}
    ```

  * 示例代码
  
    ```rust
    use clickhouse_rs::Pool;

    #[tokio::main]
    async fn main() {
        let database_url="tcp://username:password@service-terrabase-9s29mdlsb7.terrabase-9s2mdsb-hb-public.jvessel2.jdcloud.com:9000?compression=lz4".to_string();
        let pool = Pool::new(database_url);
    
        let mut client = pool.get_handle().await.unwrap();
        let sql = "show databases;";
        let r = client.query(sql).fetch_all().await;
        println!("result is: {:?}", r);
    }

    ```

* http 连接

  * 依赖 crate
  
    ```toml
    # clickhouse http
    clickhouse = {git = "https://github.com/loyd/clickhouse.rs", features = ["test-util"]}
    ```

  * 示例代码

    ```rust
    use clickhouse::Client;
    use clickhouse::Row;
    use serde::{Deserialize, Serialize};
    
    #[derive(Debug, Row, Serialize, Deserialize)]
    struct Database {
        name: String,
    }
    
    #[tokio::main]
    async fn main() {
        let client = Client::default()
        .with_url("https://service-terrabase-9s29mdlsb7.terrabase-9sdlb7-hb-public.jvessel2.jdcloud.com:8123")
        .with_user("username")
        .with_password("password");
    
        let sql = "SHOW databases";
        let r = client.query(sql).fetch_all::<Database>().await;
        println!("result is: {:?}", r);
    }
    ```

测试过程中，遇到了依赖冲突问题，clickhouse-rs 和 clickhouse.rs 同时依赖了 clickhouse-rs-cityhash-sys 但依赖方式不同，导致编译不通过，下一期详细聊聊踩坑及爬坑过程。
再次感谢京东云数据库团队提供的 starwift 测试实例。
下期见。