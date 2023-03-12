<div align=center>
<img src="./img/kufu.png">
</div>

> 项目尚在开发中，喜欢的同学可以点点 star、点点 watch 关注项目进度。

## 什么是 Kufu

Kufu 读起来像是功夫的英文发音，由 Kubernetes 和 Fuse（用户文件系统）拼接而成。顾名思义 Kufu 是一个基于Kubernetes 和 Fuse 用户文件系统开发的开源项目。它可以实时同步 Kubernetes集群内的原生（Pod、Service 等）或 CRD 资源到使用者的本地文件系统，让用户可以像操作文件系统一样操作自己的Kubernetes 集群，无需再费力的查找和执行 kubectl 指令。

## 基本原理

Kufu 利用 [kube](https://github.com/kube-rs/kube) 项目监听 Kubernetes 集群的资源变动，实时同步到使用 [sled](https://github.com/spacejam/sled)（Rust编写的本地kv数据库）中。（这里和传统的控制器开发模式不一致的地方是：Kufu 仅仅 list-watch 资源变化，但是没有把监听对象缓存到内存中，这里是考虑到 Kufu 的使用环境一般为开发者的本地电脑，尽量不占用开发者电脑的内存。）

使用 [fuser](https://github.com/cberner/fuser) 库挂载用户文件系统到用户的电脑里，每次用户对该用户文件系统内的文件的操作都会映射为读取 sled 数据库内的内容并返回给用户。

## 使用方式

[![asciicast](https://asciinema.org/a/566722.svg)](https://asciinema.org/a/566722)

## 本地开发

### MacOs

1. 安装 [MacFuse](https://osxfuse.github.io/)
2. 克隆项目
    ```shell
    git clone https://github.com/yangsoon/kufu.git
    ```
3. 修改 `test/config` 文件
    ```yaml
    mount:
    path: ./test/k8s # 指定用户文件系统的挂载位置
    data-path: ./test/.data # sled数据库数据存储位置
    resources:     # 指定资源监听类型
        - apiVersion: v1
          kind: Pod
        - apiVersion: v1
          kind: Namespace
    kube-configs:  # 指定监听集群 kubeconfig 位置
        - config-path: ~/.kube/config
    ```
4. 本地测试运行
    ```shell
    cargo run
    ```

## 待做事项

- [x] POC 验证通过
- [ ] 增加 workqueue 机制保证资源变更能够最终存储到本地
- [ ] 完善 fuse 接口剩余接口实现
- [ ] 支持更多原生资源的监听
- [ ] 完善权限系统
- [ ] 支持权限系统

