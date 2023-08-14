# 文盘Rust -- 配置文件解析

处理配置文件是应用开发的常规操作。成熟的开发语言都有自己处理配置文件的套路。golang 有 viper 这样的成熟第三方库来处理配置文件。rust 的第三方库并不成熟。
这篇文章我们来聊聊 rust 如何处理配置文件。

## 处理yaml配置文件的流程

配置文件的作用是一系列应用程序相应功能的开关。在应用启动前配置，应用启动时加载，以备运行时使用。
我们依旧用[interactcli-rs](https://github.com/jiashiwen/interactcli-rs) 为例，说明一下配置文件的处理过程。
解析配置文件的主要逻辑在 src/configure 目录。

* 定义 config 结构体
首先，定义一个结构体用来承载配置项。由于 Config struct 需要与yaml文件交互，我们定义一个具备序列化与反序列化能力的结构体

```rust
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Config {
    pub server: String,
    pub token: String,
}
```

* 为 Config 结构体定义必要的功能

```rust
impl Config {
    pub fn default() -> Self {
        Self {
            server: "http://127.0.0.1:8080".to_string(),
            token: "".to_string(),
        }
    }

    pub fn set_self(&mut self, config: Config) {
        self.server = config.server;
        self.token = config.token;
    }

    pub fn get_config_image(&self) -> Self {
        self.clone()
    }

    pub fn flush_to_file(&self, path: String) -> Result<()> {
        let yml = serde_yaml::to_string(&self)?;
        fs::write(path, yml)?;
        Ok(())
    }
}

```

* 利用 lazy_static 初始化配置项单例
  
```rust
lazy_static::lazy_static! {
    static ref GLOBAL_CONFIG: Mutex<Config> = {
        let global_config = Config::default();
        Mutex::new(global_config)
    };
    static ref CONFIG_FILE_PATH: RwLock<String> = RwLock::new({
        let path = "".to_string();
        path
    });
}
```

* 加载配置文件
[interactcli-rs](https://github.com/jiashiwen/interactcli-rs) 是一个命令行程序。加载配置文件的策略为：当指定配置文件位置时，则按给定路径加载配置；如果未指定配置文件则按照默认路径加载，此时若默认配置文件不存在则终止程序。
src/cmd/rootcmd.rs 中的 cmd_match 函数包含上面的逻辑。

```rust
fn cmd_match(matches: &ArgMatches) {
    if let Some(c) = matches.value_of("config") {
        set_config_file_path(c.to_string());
        set_config_from_file(&get_config_file_path());
    } else {
        set_config_from_file("");
    }
    ......
```

## 后记

手工处理配置文件还是比较繁琐。尤其在配置文件的书写上，必须明确配置每一个配置项，即使配置项为空也需填写。为了保证配置文件的配置项齐全，我们为Config struct 定义了 flush_to_file 函数，用来生成配置文件。
由于rust的生态较 golang 以及 java 的生态还很年轻，第三方的工具包不及两者完善。在配置文件的处理上比较繁琐，很多地方需要手工处理，但是已基本满足要求。
咱们下期见。