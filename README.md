<div align=center>
<img src="./img/kufu.png">
</div>

> The project is currently under development, and interested users can star and watch to stay updated on its progress.

[中文](./README-ZH.md)

## What is Kufu

Kufu has a pronunciation similar to "kung fu" in English, and is a combination of Kubernetes and Fuse (user file system). As its name suggests, Kufu is an open-source project based on Kubernetes and Fuse user file system. It can synchronize native resources (Pod, Service, etc.) or CRD resources in Kubernetes cluster in real time to the user's local file system, allowing users to operate their Kubernetes cluster like a file system without having to search for and execute kubectl commands.

## Basic Principles

Kufu uses the [kube](https://github.com/kube-rs/kube)  project to listen for changes in Kubernetes cluster resources and synchronizes them in real time to [sled](https://github.com/spacejam/sled) (a local KV database written in Rust). The difference from the traditional controller development mode is that Kufu only list-watch resource changes but does not cache the monitored objects in memory, as Kufu is generally used on developers' local computers and tries not to occupy too much memory.

The  [fuser](https://github.com/cberner/fuser) library is used to mount the user file system to the user's computer, and each operation on the files in this file system is mapped to reading the contents of sled database and returned to the user.

## Usage

[![asciicast](https://asciinema.org/a/566722.svg)](https://asciinema.org/a/566722)

### Local Development
#### MacOs
1. Install [MacFuse](https://osxfuse.github.io/)
2. Clone the project
    ```shell
    git clone https://github.com/yangsoon/kufu.git
    ```
3. Modify the `test/config` file

    ```yaml
    mount:
        path: ./test/k8s # Specify the mount location of the user file system
        data-path: ./test/.data # Location to store sled database data
    resources: # Specify the resource monitoring types
        - apiVersion: v1
          kind: Pod
        - apiVersion: v1
          kind: Namespace
    kube-configs: # Specify the kubeconfig location of the monitored cluster
        - config-path: ~/.kube/config
    ```
4. Run the local
    ```shell
    cargo run
    ```

## TODO
 - [x] Verify POC
 - [ ] Add workqueue mechanism to ensure that resource changes can eventually be stored locally
 - [ ] Complete remaining fuse interface implementations
 - [ ] Support monitoring of more native resources
 - [ ] Improve the permission system
 - [ ] Support permission system?