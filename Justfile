cert-manager-version := "v1.17.1"


create-cluster:
    kind create cluster --config="{{justfile_directory()}}/config.yaml"

install-cert-manager:
    kubectl apply -f 'https://github.com/cert-manager/cert-manager/releases/download/{{ cert-manager-version }}/cert-manager.yaml'

setup-cluster: create-cluster install-cert-manager

delete-cluster:
    kind delete cluster
