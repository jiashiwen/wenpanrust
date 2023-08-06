
vector 配置

```toml
[sources.inputlog]
type = "file"
data_dir =  "/root/genlogs/logs"
include = [ "/root/genlogs/logs/*.log" ]
#glob_minimum_cooldown_ms = 1_000
read_from =  "beginning"
#ignore_checkpoints = true

[sinks.out]
inputs = ["inputlog"]
type = "console"
encoding.codec = "text"
target = "stdout"
```