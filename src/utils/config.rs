use kube::config::{KubeConfigOptions, Kubeconfig};

pub fn get_current_kubeconfig_options(kubeconfig: &Kubeconfig) -> KubeConfigOptions {
    let context_detail = kubeconfig
        .contexts
        .iter()
        .find(|&c| c.name.eq(&kubeconfig.current_context.clone().unwrap()))
        .map(|c| c.to_owned());
    let cluster = context_detail.map(|c| c.context.unwrap());
    KubeConfigOptions {
        context: kubeconfig.current_context.clone(),
        cluster: cluster.clone().map(|c| c.cluster),
        user: cluster.map(|c| c.cluster),
    }
}
