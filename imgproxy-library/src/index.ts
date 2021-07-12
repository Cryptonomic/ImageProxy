import fetch from "node-fetch";
import {
  ImageProxyServer,
  FetchResponse,
  ImageProxyError,
  FetchRequest,
  ImageProxyRequest,
  ImageProxyMethod,
  ImageProxyDataType,
  ReportRequest,
  ReportResponse,
  DescribeRequest,
  DescribeResponse,
  DescribeReportsResponse,
  DescribeReportsRequest,
  ModerationLabel,
  ImageProxyResponse,
} from "./types";
export * from "./types";

function send(server: ImageProxyServer, request: ImageProxyRequest) {
  const options = {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      apikey: server.apikey,
    },
    body: JSON.stringify(request),
  };
  return fetch(server.url, options);
}

export async function proxyFetch(
  server: ImageProxyServer,
  url: string,
  response_format: ImageProxyDataType,
  force: boolean
): Promise<FetchResponse | ImageProxyError> {
  const fetchRequest: FetchRequest = {
    jsonrpc: server.version,
    method: ImageProxyMethod.Fetch,
    params: {
      data: response_format,
      url,
      force,
    },
  };

  const response = await send(server, fetchRequest);
  const parsed = response
    .json()
    .then((json) => json)
    .catch((_) => response.text())
    .then((text) => text);
  return parsed;
}

export async function safeFetch(
  server: ImageProxyServer,
  url: string,
  response_format: ImageProxyDataType
): Promise<FetchResponse | ImageProxyError> {
  return proxyFetch(server, url, response_format, false);
}

export async function unsafeFetch(
  server: ImageProxyServer,
  url: string,
  response_format: ImageProxyDataType
): Promise<FetchResponse | ImageProxyError> {
  return proxyFetch(server, url, response_format, true);
}

export async function describe(
  server: ImageProxyServer,
  urls: string[]
): Promise<DescribeResponse | ImageProxyError> {
  const describeRequest: DescribeRequest = {
    jsonrpc: server.version,
    method: ImageProxyMethod.Describe,
    params: { urls },
  };
  return send(server, describeRequest).then((response) => response.json());
}

export async function report(
  server: ImageProxyServer,
  url: string,
  categories: ModerationLabel[]
): Promise<ReportResponse | ImageProxyError> {
  const reportRequest: ReportRequest = {
    jsonrpc: server.version,
    method: ImageProxyMethod.Report,
    params: { url, categories },
  };
  return send(server, reportRequest).then((response) => response.json());
}

export async function describeReports(
  server: ImageProxyServer
): Promise<DescribeReportsResponse | ImageProxyError> {
  const describeReportsRequest: DescribeReportsRequest = {
    jsonrpc: server.version,
    method: ImageProxyMethod.DescribeReports,
  };
  return send(server, describeReportsRequest).then((response) =>
    response.json()
  );
}
