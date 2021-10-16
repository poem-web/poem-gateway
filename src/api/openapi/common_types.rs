use poem_openapi::{
    payload::Json,
    types::{ParseFromJSON, ToJSON, Type},
    ApiResponse,
};

#[derive(ApiResponse)]
pub enum CreateResourceResponse {
    #[oai(status = 200)]
    Ok(Json<String>),
}

#[derive(ApiResponse)]
pub enum UpdateResourceResponse {
    #[oai(status = 200)]
    Ok,
}

#[derive(ApiResponse)]
pub enum GetResourceResponse<T: Type + ParseFromJSON + ToJSON> {
    #[oai(status = 200)]
    Ok(Json<T>),
    #[oai(status = 404)]
    NotFound,
}

#[derive(ApiResponse)]
pub enum GetResourcesResponse<T: Type + ParseFromJSON + ToJSON> {
    #[oai(status = 200)]
    Ok(Json<Vec<T>>),
}

#[derive(ApiResponse)]
pub enum DeleteResourceResponse {
    #[oai(status = 200)]
    Ok,
    #[oai(status = 404)]
    NotFound,
}
