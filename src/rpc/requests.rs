use serde::Deserialize;

use crate::moderation::ModerationCategories;

#[derive(Debug, Deserialize)]
#[allow(non_camel_case_types)]
pub enum RpcMethods {
    img_proxy_fetch,
    img_proxy_describe,
    img_proxy_report,
    img_proxy_describe_report,
}
// RPC Header
#[derive(Deserialize)]
pub struct MethodHeader {
    pub jsonrpc: String,
    pub method: RpcMethods,
}

// Fetch method struct
#[derive(Deserialize)]
pub struct FetchRequestParams {
    pub url: String,
    pub force: bool,
}

#[derive(Deserialize)]
pub struct FetchRequest {
    pub params: FetchRequestParams,
}

#[derive(Deserialize)]
pub struct DescribeRequestParams {
    pub urls: Vec<String>,
}

#[derive(Deserialize)]
pub struct DescribeRequest {
    pub params: DescribeRequestParams,
}

#[derive(Deserialize)]
pub struct ReportRequestParams {
    pub url: String,
    pub categories: Vec<ModerationCategories>,
}

#[derive(Deserialize)]
pub struct ReportRequest {
    pub params: ReportRequestParams,
}
