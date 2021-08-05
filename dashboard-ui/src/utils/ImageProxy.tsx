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

export const findMetric = (arr: any, name: string, label = "name") =>
  arr?.find((metric: any) => metric[label] === name);

export const findNestedMetric = (
  metrics: any,
  name: string,
  labelName: string,
  label: string
) =>
  findMetric(metrics, name)?.metrics.find(
    (elem: any) => elem.labels[labelName] === label
  );

export const findCacheMetrics = (metrics: any, name: string) =>
  findNestedMetric(metrics, "cache_metrics", "metric", name);

export const findApiMetrics = (metrics: any, name: string) => {
  const i = findNestedMetric(metrics, "api_requests", "rpc_method", name);
  return i ? parseInt(i.value) : 0;
};

export const findApiResponseTimeMetrics = (metrics: any) => {
  const responseTimes = findMetric(metrics, "api_response_time").metrics[0]
    .buckets;
  const unparsed = Object.keys(responseTimes)
    .map((name) => ({
      name,
      value: parseInt(responseTimes[name]),
    }))
    .sort((d) => d.value);
  return unparsed
    .map((elem, i) =>
      i === 0 ? elem : { ...elem, value: elem.value - unparsed[i - 1].value }
    )
    .sort((d) => parseInt(d.name));
};

export const secondsToHMS = (d: number) => {
  const h = Math.floor(d / 3600);
  const m = Math.floor((d % 3600) / 60);
  const s = (d % 3600) % 60;

  const hDisplay = h > 0 ? h + (h === 1 ? " hr, " : " hrs, ") : "";
  const mDisplay = m > 0 ? m + (m === 1 ? " min, " : " mins, ") : "";
  const sDisplay = s > 0 ? s.toFixed(3) + (s === 1 ? " sec" : " secs") : "";
  return hDisplay + mDisplay + sDisplay;
};
