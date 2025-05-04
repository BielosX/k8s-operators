cert-manager-version := "v1.17.1"
prometheus-operator-version := "v0.82.0"
grafana-operator-version := "v5.17.1"
ingress-nginx-version := "v1.12.2"


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

install-ingress-nginx:
    kubectl apply -f 'https://raw.githubusercontent.com/kubernetes/ingress-nginx/controller-{{ ingress-nginx-version }}/deploy/static/provider/cloud/deploy.yaml'
    kubectl rollout status -w -n ingress-nginx deployment/ingress-nginx-controller --timeout=120s
    kubectl apply -f nginx_internal.yaml

setup-cluster: create-cluster install-cert-manager install-prometheus-operator install-ingress-nginx

delete-cluster:
    kind delete cluster
