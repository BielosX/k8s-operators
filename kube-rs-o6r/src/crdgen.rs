mod exposed_app;
use exposed_app::ExposedApp;
use kube::CustomResourceExt;

fn main() {
    print!("{}", serde_yaml::to_string(&ExposedApp::crd()).unwrap())
}
