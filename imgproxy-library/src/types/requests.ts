import { ModerationLabel } from "./common";

export enum ImageProxyMethod {
  Fetch = "img_proxy_fetch",
  Describe = "img_proxy_describe",
  Report = "img_proxy_report",
  DescribeReports = "img_proxy_describe_report",
}

export enum ImageProxyDataType {
  Raw = "Raw",
  Json = "Json",
}

interface RequestHeader {
  jsonrpc: string;
  method: ImageProxyMethod;
}

interface FetchParams {
  params: {
    response_type: ImageProxyDataType;
    url: string;
    force: boolean;
  };
}

interface DescribeParams {
  params: {
    urls: string[];
  };
}

interface ReportParams {
  params: {
    url: string;
    categories: ModerationLabel[];
  };
}

export type FetchRequest = RequestHeader & FetchParams;
export type DescribeRequest = RequestHeader & DescribeParams;
export type ReportRequest = RequestHeader & ReportParams;
export type DescribeReportsRequest = RequestHeader;
