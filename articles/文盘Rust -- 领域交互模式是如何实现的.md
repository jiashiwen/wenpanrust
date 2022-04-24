# 文盘Rust -- 领域交互模式如何实现

书接上文，上回说到如何通过[interactcli-rs](https://github.com/jiashiwen/interactcli-rs)四步实现一个命令行程序。但是shell交互模式在有些场景下用户体验并不是很好。比如我们要连接某个服务，比如mysql或者redis这样的服务。如果每次交互都需要输入地址、端口、用户名等信息，加护起来太麻烦。通常的做法是一次性输入和连接相关的信息或者由统一配置文件进行管理，然后进入领域交互模式，所有的命令和反馈都和该领域相关。[interactcli-rs](https://github.com/jiashiwen/interactcli-rs) 通过 -i 参数实现领域交互模式。这回我们探索一下这一模式是如何实现的。

## 基本原理
interactcli-rs 实现领域交互模式主要是循环解析输入的每一行，通过[rustyline](https://github.com/kkawakam/rustyline) 解析输入的每一行命令，并交由命令解析函数处理响应逻辑

当我们调用 ‘-i’ 参数的时候 实际上是执行了 interact::run() 函数(interact -> cli -> run())。

```rust
pub fn run() {
    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .output_stream(OutputStreamType::Stdout)
        .build();

    let h = MyHelper {
        completer: get_command_completer(),
        highlighter: MatchingBracketHighlighter::new(),
        hinter: HistoryHinter {},
        colored_prompt: "".to_owned(),
        validator: MatchingBracketValidator::new(),
    };

    let mut rl = Editor::with_config(config);
    rl.set_helper(Some(h));

    if rl.load_history("/tmp/history").is_err() {
        println!("No previous history.");
    }

    loop {
        let p = format!("{}> ", "interact-rs");
        rl.helper_mut().expect("No helper").colored_prompt = format!("\x1b[1;32m{}\x1b[0m", p);
        let readline = rl.readline(&p);
        match readline {
            Ok(line) => {
                if line.trim_start().is_empty() {
                    continue;
                }

                rl.add_history_entry(line.as_str());
                match split(line.as_str()).as_mut() {
                    Ok(arg) => {
                        if arg[0] == "exit" {
                            println!("bye!");
                            break;
                        }
                        arg.insert(0, "clisample".to_string());
                        run_from(arg.to_vec())
                    }
                    Err(err) => {
                        println!("{}", err)
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    rl.append_history("/tmp/history")
        .map_err(|err| error!("{}", err))
        .ok();
}
```

## 命令行解析主逻辑

交互逻辑主要集中在 ‘loop’ 循环中，每次循环处理一次输入请求。

处理的逻辑如下

* 定义提示符，类似 'mysql> ',提示用户正在使用的程序

```rust
 let p = format!("{}> ", "interact-rs");
 rl.helper_mut().expect("No helper").colored_prompt = format!("\x1b[1;32m{}\x1b[0m", p);
```

* 读取输入行进行解析
  * 将输入的命令行加入到历史文件，执行过的命令可以通过上下键回放来增强用户体验。
  
  ```rust
  rl.add_history_entry(line.as_str());
  ```

  * 将输入的行解析为 arg 字符串，交由 cmd::run_from 函数进行命令解析和执行

  ```rust
  match split(line.as_str()).as_mut() {
                    Ok(arg) => {
                        if arg[0] == "exit" {
                            println!("bye!");
                            break;
                        }
                        arg.insert(0, "clisample".to_string());
                        run_from(arg.to_vec())
                    }
                    Err(err) => {
                        println!("{}", err)
                    }
                }
  ```

  * 解析中断，当用户执行 ctrl-c 或 ctrl-d 时，退出程序。
  
  ```rust
     Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
  ```

## run 函数中其他代码的作用

* 配置rustyline
  在 run 函数最开头 定义了一个config

  ```rust
  let config = Config::builder()
    .history_ignore_space(true)
    .completion_type(CompletionType::List)
    .output_stream(OutputStreamType::Stdout)
    .build();
  ```
  这个config 其实是rustyline的配置项，包括输出方式历史记录约束，输出方式等等。

  MyHelper 用于配置命令的 autocomplete

  ```rust
  let h = MyHelper {
    completer: get_command_completer(),
    highlighter: MatchingBracketHighlighter::new(),
    hinter: HistoryHinter {},
    colored_prompt: "".to_owned(),
    validator: MatchingBracketValidator::new(),
  }; 
  ```
  这里卖个关子，下期详细讲讲 autocomplete 的实现。

* 配置历史文件
  run 函数最后，我们为程序配置了历史文件，应用于存放执行过的历史命令。这样即便程序退出，在此打开程序的时候还是可以利用以前的执行历史。

  ```rust
  rl.append_history("/tmp/history")
        .map_err(|err| error!("{}", err))
        .ok();
  ```

关于如何构建命令行的领域交互模式就说到这儿，下期详细介绍一下 autocomplete 如何实现。
