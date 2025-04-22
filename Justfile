cert-manager-version := "v1.17.1"
prometheus-operator-version := "v0.82.0"
grafana-operator-version := "v5.17.1"


create-cluster:
    kind create cluster --config="{{justfile_directory()}}/config.yaml"

install-cert-manager:
    kubectl apply -f 'https://github.com/cert-manager/cert-manager/releases/download/{{ cert-manager-version }}/cert-manager.yaml'
    cmctl check api --wait=2m

install-prometheus-operator:
    curl -sL 'https://github.com/prometheus-operator/prometheus-operator/releases/download/{{ prometheus-operator-version }}/bundle.yaml' | kubectl create -f -
    kubectl wait --for=condition=Ready pods -l app.kubernetes.io/name=prometheus-operator
    kubectl apply -f prometheus.yaml
    kubectl apply -f prometheus_service.yaml

install-grafana-operator:
    kubectl create -f 'https://github.com/grafana/grafana-operator/releases/download/{{ grafana-operator-version }}/kustomize-cluster_scoped.yaml'
    kubectl wait --timeout=2m --for=condition=Ready pods -n grafana -l app.kubernetes.io/name=grafana-operator

setup-cluster: create-cluster install-cert-manager install-prometheus-operator install-grafana-operator

delete-cluster:
    kind delete cluster
