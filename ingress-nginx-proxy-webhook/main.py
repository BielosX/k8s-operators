from fastapi import FastAPI, Request

app = FastAPI()

@app.get("/healthz")
def health():
    return "OK"

def review_response(uid: str, allowed: bool, code=200, message=""):
    return {
        "apiVersion": "admission.k8s.io/v1",
        "kind": "AdmissionReview",
        "response": {
            "uid": uid,
            "allowed": allowed,
            "status": {
                "code": code,
                "message": message,
            }
        }
    }

@app.post("/validate")
async def validate(request: Request):
    body = await request.json()
    uid = body["request"]["uid"]
    object = body["request"]["object"]
    containers = object["spec"]["template"]["spec"]["containers"]
    for container in containers:
        tag = container["image"].split(":")[1]
        if tag == "latest":
            return review_response(uid, False, 400, "latest tag is not allowed")
    return review_response(uid, True)