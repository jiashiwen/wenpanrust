---
marp: true
size: 4:3
# paginate: true
theme: default
---

# Rust 如何在中小团队落地

---

## 自我介绍

贾世闻 
京东云架构师
资深开源贡献者
《文盘Rust》专栏作者
Rust一介散修

---

## rust 热度为何逐年攀升

* 语言特性，安全、无GC、高性能 -> 程序员喜欢
* 大厂加持 -> 老板喜欢 --为啥喜欢，
* 生态不断成长 -> 社区喜欢 --为什么

---

## 从高级语言发展历史看rust前景

* c 解决平台移至问题
* java 解决内存管理问题
* python 等解释型语言解决开发效率问题
* golang 静态语言安全与动态语言易开发性相结合
* rust 无GC+内存安全+高性能

rust的差异性
  
---

## rust 对企业的正向意义

* 安全 Cloudflare 使用rust 替换 NGINX/OpenResty 代理的组件
* 减碳 
  《Energy Efficiency across Programming Languages》
  https://greenlab.di.uminho.pt/wp-content/uploads/2017/10/sleFinal.pdf  
* 降本
  
---

## 哪些应用后端场景有rust的基础组件

* web 后端框架 hyper、Axum、Warp
* 权限框架 casbin-rs
* ORM SeaORM、SQLx、rbatis
* 原生数据库驱动 Mysql、Postgresql、sqlite、MSSQL
* 搜索引擎 MeiliSearch、Tantivy

---

## 我们团队的故事

* redissyncer
  https://github.com/TraceNature/redissyncer-server
* 客户端用以及场景测试程序 rust 替代 golang
* 收益
  * 抽象了较为通用的cli框架
  * 解决了golang cli 程序退格闪烁的问题
  * 在这个过程中积累了一篇rust实践文档，以《文盘Rust》系列技术文章的形式输出

---

## 遗留项目用rust改造需要主意哪些问题

* 基础组件生态是否对齐
* 基础库成熟度是否能够满足要求
* 工程构建时效性是否满足项目要求
* 循序渐进，逐步改造，不要 All in

---

## 使用rust要做好的心理准备

* 无成熟框架，应用构建全手动
* 大部分库成熟度仍有待提高
* 应用开发效率卷不过java 和 go

