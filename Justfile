create-cluster:
    kind create cluster --config="{{justfile_directory()}}/config.yaml"

delete-cluster:
    kind delete cluster