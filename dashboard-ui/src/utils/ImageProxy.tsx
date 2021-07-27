import config from "../config.json";
import parsePrometheusTextFormat from "parse-prometheus-text-format";
import {
  DescribeResponse,
  FetchResponse,
  ImageProxyError,
  ImageProxyServer,
} from "nft-image-proxy";

export interface BuildInfo {
  package_version: string;
  git_version: string;
}

const server: ImageProxyServer = {
  url: config.proxyURL,
  version: "1.0.0",
  apikey: config.apikey,
};

export const getInfo = async (): Promise<BuildInfo> => {
  return (await fetch(`${config.proxyURL}/info`)).json();
};

export const getReports = async (): Promise<
  DescribeResponse | ImageProxyError
> => {
  const init = {
    method: "POST",
    body: JSON.stringify({
      jsonrpc: "1.0.0",
      method: "img_proxy_describe_report",
    }),
    headers: {
      apikey: config.apikey,
    },
  };
  const res = (
    await fetch(`${config.proxyURL}`, init).then((res) => res.json())
  ).result;
  return res;
};

export const getModerationReports = async (): Promise<
  FetchResponse | ImageProxyError
> => {
  const init = {
    method: "POST",
    body: JSON.stringify({
      jsonrpc: "1.0.0",
      method: "img_proxy_describe",
      params: {
        urls: ["*"],
      },
    }),
    headers: {
      apikey: config.apikey,
    },
  };
  const res = await fetch(`${config.proxyURL}`, init).then((res) =>
    res
      .json()
      .then((json) =>
        json.result.map((entry: any, i: number) => ({ ...entry, index: i }))
      )
  );

  return res;
};

export const getMetrics = async () => {
  return fetch(`${config.proxyURL}/metrics`).then((d) =>
    d.text().then((raw) => parsePrometheusTextFormat(raw))
  );
};
