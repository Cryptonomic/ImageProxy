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
} from "./types";

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

/**
 * Returns a fetch reponse from specified image proxy with the image data in it
 * @param {Server} server - Server configuration object
 * @param {string} url - Url of the image to fetch
 * @param {ImageProxyDataType} response_type - Data type of response. Either raw bytes or json response
 * @param {boolean} force - Whether or not image should be returned if it is potentially blocked
 * @returns {Promise<FetchResponse | ImageProxyError>} Image proxy reponse or error
 */
export async function proxyFetch(
  server: ImageProxyServer,
  url: string,
  response_type: ImageProxyDataType,
  force: boolean
): Promise<FetchResponse | ImageProxyError> {
  const fetchRequest: FetchRequest = {
    jsonrpc: server.version,
    method: ImageProxyMethod.Fetch,
    params: {
      response_type,
      url,
      force,
    },
  };
  const response = (await send(server, fetchRequest)).text().then((text) => {
    try {
      return JSON.parse(text);
    } catch (e) {
      return text;
    }
  });
  return response;
}

/**
 * Returns a moderated fetch reponse from specified image proxy with the image data in it, blocking unsafe images
 * @param {Server} server - Server configuration object
 * @param {string} url - Url of the image to fetch
 * @param {ImageProxyDataType} response_type - Data type of response. Either raw bytes or json response
 * @returns {Promise<FetchResponse | ImageProxyError>} Image proxy reponse or error
 */
export async function safeFetch(
  server: ImageProxyServer,
  url: string,
  response_format: ImageProxyDataType
): Promise<FetchResponse | ImageProxyError> {
  return proxyFetch(server, url, response_format, false);
}

/**
 * Returns an unmoderated fetch reponse from specified image proxy with the image data in it, allowing all images
 * @param {Server} server - Server configuration object
 * @param {string} url - Url of the image to fetch
 * @param {ImageProxyDataType} response_type - Data type of response. Either raw bytes or json response
 * @returns {Promise<FetchResponse | ImageProxyError>} Image proxy reponse or error
 */
export async function unsafeFetch(
  server: ImageProxyServer,
  url: string,
  response_format: ImageProxyDataType
): Promise<FetchResponse | ImageProxyError> {
  return proxyFetch(server, url, response_format, true);
}

/**
 * Describes the moderation status of provided urls
 * @param {Server} server - Server configuration object
 * @param {string} urls - List of urls of the images to describe
 * @returns {Promise<DescribeResponse | ImageProxyError>} Describe reponse or error
 */
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

/**
 * Report a url that was deemed safe but should be blocked
 * @param {Server} server - Server configuration object
 * @param {string} urls - List of urls of the images to report
 * @param {ModerationLabel[]} categories - List of moderation labels to report the image as
 * @returns {Promise<ReportResponse | ImageProxyError>} Report reponse or error
 */
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

/**
 * Describe all reports submitted to image proxy
 * @param {Server} server - Server configuration object
 * @returns {Promise<DescribeReportResponse | ImageProxyError>} Describe report response or error
 */
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
