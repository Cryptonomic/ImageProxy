import config from "../config.json";

export interface Report {
  [key: string]: any;
  categories: string[];
  url: string;
  id: string;
  updated_at: string;
}

export interface Info {
  package_version: string;
  git_version: string;
}

export const getReports = async (): Promise<Report[]> => {
  const init = {
    method: "POST",
    body: JSON.stringify({
      jsonrpc: "1.0.0",
      method: "img_proxy_describe_report",
    }),
  };
  const res: Report[] = (
    await fetch(`${config.proxyURL}`, init).then((res) => res.json())
  ).result;
  return res;
  /*   
  return [
    {
      index: 1,
      reason: ["nudity"],
      url: "ipfs://ipfs/QmaaSqjrvJXMNyRsHHDrCfWBX6LJmdMR85qjQmd1rf8rie",
      num_reports: 2,
      last_time: "1-1-20",
      labels: ["nudity", "graphic nudity"],
    },
    {
      index: 2,
      reason: ["nudity"],
      url: "ipfs://ipfs/QmP9ymAjLyxM6gHHVnVDp3PYFtbx4KbaC12qH991GcgPNW",
      num_reports: 5,
      last_time: "2-2-20",
      labels: ["nudity", "graphic nudity"],
    },
    {
      index: 3,
      reason: ["graphic violence"],
      url: "ipfs://ipfs/Qmb21ShLn2CLDTy5cYbXPS3yZVPmiCVH8RXzBUcXAp4qhu",
      num_reports: 42,
      last_time: "1-2-20",
      labels: ["graphic content"],
    },
  ]; */
};

export const getInfo = async (): Promise<Info> => {
  return (await fetch(`${config.proxyURL}/${config.infoEndpoint}`)).json();
};
