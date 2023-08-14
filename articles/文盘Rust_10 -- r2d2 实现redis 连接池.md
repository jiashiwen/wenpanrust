# 文盘Rust --  r2d2 实现redis 连接池

我们在开发应用后端系统的时候经常要和各种数据库、缓存等资源打交道。这一期，我们聊聊如何访问redis 并将资源池化。
在一个应用后端程序访问redis主要要做的工作有两个，单例和池化。

在后端应用集成redis，我们主要用到一下几个crate:[once_cell](https://github.com/matklad/once_cell)、[redis-rs](https://github.com/redis-rs/redis-rs)、[r2d2](https://github.com/sfackler/r2d2).once_cell 实现单例；redis-rs 是 redis的 rust 驱动；r2d2 是一个池化连接的工具包.本期代码均出现在[fullstack-rs](https://github.com/jiashiwen/fullstack-rs)项目中。[fullstack-rs](https://github.com/jiashiwen/fullstack-rs)是我新开的一个实验性项目，目标是做一个类似[gin-vue-admin](https://github.com/flipped-aurora/gin-vue-admin)的集成开发框架。

redis 资源的定义主要是在https://github.com/jiashiwen/fullstack-rs/blob/main/backend/src/resources/redis_resource.rs 中实现的。

* redis-rs 封装
  
在实际开发中，我们面对的redis资源可能是单实例也有可能是集群，在这里我们对redis-rs进行了简单封装，便于适应这两种情况。

```rust
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub struct RedisInstance {
    #[serde(default = "RedisInstance::urls_default")]
    pub urls: Vec<String>,
    #[serde(default = "RedisInstance::password_default")]
    pub password: String,
    #[serde(default = "RedisInstance::instance_type_default")]
    pub instance_type: InstanceType,
}
```

RedisInstance,定义redis资源的描述，与配置文件相对应。详细的配置描述可以参考 https://github.com/jiashiwen/fullstack-rs/blob/main/backend/src/configure/config_global.rs 文件中 RedisConfig 和 RedisPool 两个 struct 描述。

```rust
#[derive(Clone)]
pub enum RedisClient {
    Single(redis::Client),
    Cluster(redis::cluster::ClusterClient),
}

impl RedisClient {
    pub fn get_redis_connection(&self) -> RedisResult<RedisConnection> {
        return match self {
            RedisClient::Single(s) => {
                let conn = s.get_connection()?;
                Ok(RedisConnection::Single(Box::new(conn)))
            }
            RedisClient::Cluster(c) => {
                let conn = c.get_connection()?;
                Ok(RedisConnection::Cluster(Box::new(conn)))
            }
        };
    }
}

pub enum RedisConnection {
    Single(Box<redis::Connection>),
    Cluster(Box<redis::cluster::ClusterConnection>),
}

impl RedisConnection {
    pub fn is_open(&self) -> bool {
        return match self {
            RedisConnection::Single(sc) => sc.is_open(),
            RedisConnection::Cluster(cc) => cc.is_open(),
        };
    }

    pub fn query<T: FromRedisValue>(&mut self, cmd: &redis::Cmd) -> RedisResult<T> {
        return match self {
            RedisConnection::Single(sc) => match sc.as_mut().req_command(cmd) {
                Ok(val) => from_redis_value(&val),
                Err(e) => Err(e),
            },
            RedisConnection::Cluster(cc) => match cc.req_command(cmd) {
                Ok(val) => from_redis_value(&val),
                Err(e) => Err(e),
            },
        };
    }
}
```

RedisClient 和 RedisConnection 对redis 的链接进行了封装，用来实现统一的调用接口。

* 基于 r2d2 实现 redis 连接池
  
以上，基本完成的reids 资源的准备工作，下面来实现一个redis链接池。

```rust
#[derive(Clone)]
pub struct RedisConnectionManager {
    pub redis_client: RedisClient,
}

impl r2d2::ManageConnection for RedisConnectionManager {
    type Connection = RedisConnection;
    type Error = RedisError;

    fn connect(&self) -> Result<RedisConnection, Self::Error> {
        let conn = self.redis_client.get_redis_connection()?;
        Ok(conn)
    }

    fn is_valid(&self, conn: &mut RedisConnection) -> Result<(), Self::Error> {
        match conn {
            RedisConnection::Single(sc) => {
                redis::cmd("PING").query(sc)?;
            }
            RedisConnection::Cluster(cc) => {
                redis::cmd("PING").query(cc)?;
            }
        }
        Ok(())
    }

    fn has_broken(&self, conn: &mut RedisConnection) -> bool {
        !conn.is_open()
    }
}
```

利用 r2d2 来实现连接池需要实现 r2d2::ManageConnection trait。connect 函数获取连接；is_valid 函数校验连通性；has_broken 判断连接是否崩溃不可用。

```Rust
pub fn gen_redis_conn_pool() -> Result<Pool<RedisConnectionManager>> {
    let config = get_config()?;
    let redis_client = config.redis.instance.to_redis_client()?;
    let manager = RedisConnectionManager { redis_client };
    let pool = r2d2::Pool::builder()
        .max_size(config.redis.pool.max_size as u32)
        .min_idle(Some(config.redis.pool.mini_idle as u32))
        .connection_timeout(Duration::from_secs(
            config.redis.pool.connection_timeout as u64,
        ))
        .build(manager)?;
    Ok(pool)
}
```

gen_redis_conn_pool 函数用来生成一个 redis 的连接池，根据配置文件来指定连接池的最大连接数，最小闲置连接以及连接超时时长。

* 连接池单例实现

在后端开发中，对于单一资源一般采取单例模式避免重复产生实例的开销。下面来聊一聊如果构建一个全局的 redis 资源。
这一部分代码在 https://github.com/jiashiwen/fullstack-rs/blob/main/backend/src/resources/init_resources.rs 文件中。

```rust
pub static GLOBAL_REDIS_POOL: OnceCell<r2d2::Pool<RedisConnectionManager>> = OnceCell::new();
```

利用 OnceCell 构建全局静态变量。

```rust
fn init_global_redis() {
    GLOBAL_REDIS_POOL.get_or_init(|| {
        let pool = match gen_redis_conn_pool() {
            Ok(it) => it,
            Err(err) => panic!("{}", err.to_string()),
        };
        pool
    });
}
```

init_global_redis 函数，用来初始化 GLOBAL_REDIS_POOL 全局静态变量。在一般的后端程序中，资源是强依赖，所以，初始化简单粗暴，要么成功要么 panic。

* 资源调用

准备好 redis 资源后，我们聊聊如何调用。
调用例子在这里 https://github.com/jiashiwen/fullstack-rs/blob/main/backend/src/httpserver/service/service_redis.rs

```rust
pub fn put(kv: KV) -> Result<()> {
    let conn = GLOBAL_REDIS_POOL.get();
    return match conn {
        Some(c) => {
            c.get()?
                .query(redis::cmd("set").arg(kv.Key).arg(kv.Value))?;
            Ok(())
        }
        None => Err(anyhow!("redis pool not init")),
    };
}
```

https://github.com/jiashiwen/fullstack-rs/tree/main/backend 这个工程里有从http 入口开始到写入redis的完整流程，http server 不在本文讨论之列，就不赘述了，有兴趣的同学可以去github看看。

咱们下期见。