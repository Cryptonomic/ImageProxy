import config from "../config.json";
import parsePrometheusTextFormat from "parse-prometheus-text-format";

export interface BuildInfo {
  package_version: string;
  git_version: string;
}

export const getInfo = async (): Promise<BuildInfo> => {
  return (await fetch(`${config.proxyURL}/info`)).json();
};

export const getMetrics = async () => {
  return fetch(`${config.proxyURL}/metrics`).then((d) =>
    d.text().then((raw) => parsePrometheusTextFormat(raw))
  );
};
