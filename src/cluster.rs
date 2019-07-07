use std::str;

use crate::error::BallistaError;
use k8s_openapi;
use k8s_openapi::api::core::v1 as api;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use k8s_openapi::http;
use reqwest;

fn execute(request: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>, reqwest::Error> {
    let (method, path, body) = {
        let (parts, body) = request.into_parts();
        let mut url: http::uri::Parts = parts.uri.into();
        let path = url
            .path_and_query
            .take()
            .expect("request doesn't have path and query");

        (parts.method, path, body)
    };

    let uri = format!("http://localhost:8080{}", path);

    println!("{} {}{}", method, uri, str::from_utf8(&body).unwrap());

    let client = reqwest::Client::new();

    let mut x = match method {
        http::Method::GET => client.get(&uri).body(body).send()?,
        http::Method::POST => client.post(&uri).body(body).send()?,
        http::Method::DELETE => client.delete(&uri).body(body).send()?,
        _ => unimplemented!(),
    };

    let body = x.text()?;

    println!("Response: {}", body);

    let response = http::Response::builder()
        .status(http::StatusCode::OK) //TODO translate status code or is it always OK at this point?
        .body(body.as_bytes().to_vec())
        .unwrap();

    Ok(response)
}

pub fn create_ballista_executor(namespace: &str, name: &str, image_name: &str) -> Result<(), BallistaError> {
    create_pod(namespace, name, image_name)?;
    create_service(namespace, name)
}

pub fn create_service(namespace: &str, name: &str) -> Result<(), BallistaError> {
    let mut metadata: ObjectMeta = Default::default();
    metadata.name = Some(name.to_string());

    let mut spec : api::ServiceSpec = Default::default();
    spec.type_ = Some("ClusterIP".to_string());

    let mut port : api::ServicePort = Default::default();
    port.name = Some("grpc".to_string());
    port.port = 50051;
    port.target_port = Some(IntOrString::Int(50051));

    spec.ports = Some(vec![port]);

    let mut service : api::Service = Default::default();
    service.metadata = Some(metadata);
    service.spec = Some(spec);

    let (request, response_body) =
        api::Service::create_namespaced_service(namespace, &service, Default::default())
            .expect("couldn't create service");
    let response = execute(request).expect("couldn't create service");

    // Got a status code from executing the request.
    let status_code: http::StatusCode = response.status();

    let mut response_body = response_body(status_code);
    response_body.append_slice(&response.body());
    let response = response_body.parse();

    match response {
        // Successful response (HTTP 200 and parsed successfully)
        Ok(api::CreateNamespacedServiceResponse::Ok(service)) => {
            println!("created service ok: {}", service.metadata.unwrap().name.unwrap());
            Ok(())
        }

        // Some unexpected response
        // (not HTTP 200, but still parsed successfully)
        Ok(other) => return Err(format!("expected Ok but got {} {:?}", status_code, other).into()),

        // Need more response data.
        // Read more bytes from the response into the `ResponseBody`
        Err(k8s_openapi::ResponseError::NeedMoreData) => Err(BallistaError::General(
            "Need more response data".to_string(),
        )),

        // Some other error, like the response body being
        // malformed JSON or invalid UTF-8.
        Err(err) => return Err(format!("error: {} {:?}", status_code, err).into()),
    }
}

pub fn create_pod(namespace: &str, name: &str, image_name: &str) -> Result<(), BallistaError> {
    let mut metadata: ObjectMeta = Default::default();
    metadata.name = Some(name.to_string());

    let mut container: api::Container = Default::default();
    container.name = name.to_string();
    container.image = Some(image_name.to_string());
    container.image_pull_policy = Some("Always".to_string()); //TODO make configurable

    let mut container_port: api::ContainerPort = Default::default();
    container_port.container_port = 50051;

    container.ports = Some(vec![container_port]);

    let mut pod_spec: api::PodSpec = Default::default();
    pod_spec.containers = vec![container];

    let pod = api::Pod {
        metadata: Some(metadata),
        spec: Some(pod_spec),
        status: None,
    };

    let (request, response_body) =
        api::Pod::create_namespaced_pod(namespace, &pod, Default::default())
            .expect("couldn't create pod");
    let response = execute(request).expect("couldn't create pod");

    // Got a status code from executing the request.
    let status_code: http::StatusCode = response.status();

    let mut response_body = response_body(status_code);
    response_body.append_slice(&response.body());
    let response = response_body.parse();

    match response {
        // Successful response (HTTP 200 and parsed successfully)
        Ok(api::CreateNamespacedPodResponse::Ok(pod)) => {
            println!("created pod ok: {}", pod.metadata.unwrap().name.unwrap());
            Ok(())
        }

        // Some unexpected response
        // (not HTTP 200, but still parsed successfully)
        Ok(other) => return Err(format!("expected Ok but got {} {:?}", status_code, other).into()),

        // Need more response data.
        // Read more bytes from the response into the `ResponseBody`
        Err(k8s_openapi::ResponseError::NeedMoreData) => Err(BallistaError::General(
            "Need more response data".to_string(),
        )),

        // Some other error, like the response body being
        // malformed JSON or invalid UTF-8.
        Err(err) => return Err(format!("error: {} {:?}", status_code, err).into()),
    }
}

pub fn delete_pod(namespace: &str, pod_name: &str) -> Result<(), BallistaError> {
    let (request, response_body) =
        api::Pod::delete_namespaced_pod(pod_name, namespace, Default::default())
            .expect("couldn't delete pod");
    let response = execute(request).expect("couldn't delete pod");

    // Got a status code from executing the request.
    let status_code: http::StatusCode = response.status();

    let mut response_body = response_body(status_code);
    response_body.append_slice(&response.body());
    let response = response_body.parse();

    match response {
        // Successful response (HTTP 200 and parsed successfully)
        Ok(api::DeleteNamespacedPodResponse::OkStatus(_status)) => {
            //println!("deleted pod {} ok: status {:?}", pod_name, status);
            Ok(())
        }

        Ok(api::DeleteNamespacedPodResponse::OkValue(_value)) => {
            //println!("deleted pod {} ok: value {:?}", pod_name, value);
            Ok(())
        }

        Ok(api::DeleteNamespacedPodResponse::Accepted(_status)) => {
            //println!("deleted pod {} ok: accepted status {:?}", pod_name, status);
            Ok(())
        }

        // Some unexpected response
        // (not HTTP 200, but still parsed successfully)
        Ok(other) => return Err(format!("expected Ok but got {} {:?}", status_code, other).into()),

        // Need more response data.
        // Read more bytes from the response into the `ResponseBody`
        Err(k8s_openapi::ResponseError::NeedMoreData) => Err(BallistaError::General(
            "Need more response data".to_string(),
        )),

        // Some other error, like the response body being
        // malformed JSON or invalid UTF-8.
        Err(err) => return Err(format!("error: {} {:?}", status_code, err).into()),
    }
}

pub fn list_pods(namespace: &str) -> Result<Vec<String>, BallistaError> {
    let (request, response_body) =
        api::Pod::list_namespaced_pod(namespace, Default::default()).expect("couldn't list pods");
    let response = execute(request).expect("couldn't list pods");

    // Got a status code from executing the request.
    let status_code: http::StatusCode = response.status();

    // Construct the `ResponseBody<ListNamespacedPodResponse>` using the
    // constructor returned by the API function.
    let mut response_body = response_body(status_code);

    response_body.append_slice(&response.body());

    let response = response_body.parse();
    match response {
        // Successful response (HTTP 200 and parsed successfully)
        Ok(api::ListNamespacedPodResponse::Ok(pod_list)) => {
            let mut ret = vec![];
            for pod in pod_list.items {
                ret.push(pod.metadata.unwrap().name.unwrap());
            }
            Ok(ret)
        }

        // Some unexpected response
        // (not HTTP 200, but still parsed successfully)
        Ok(other) => return Err(format!("expected Ok but got {} {:?}", status_code, other).into()),

        // Need more response data.
        // Read more bytes from the response into the `ResponseBody`
        Err(k8s_openapi::ResponseError::NeedMoreData) => Err(BallistaError::General(
            "Need more response data".to_string(),
        )),

        // Some other error, like the response body being
        // malformed JSON or invalid UTF-8.
        Err(err) => return Err(format!("error: {} {:?}", status_code, err).into()),
    }
}
