import React, { useEffect, useState } from "react";
import { getReports, Report } from "../utils/ImageProxy";
import SortByHeader from "./SortByHeader";

interface AggregatedReport {
  [key: string]: any;
  index: number;
  id: string[];
  url: string;
  categories: string[];
  updated_at: string;
  num_reports: number;
}

const aggregateReports = (reports: Report[]) => {
  let res: AggregatedReport[] = [];
  reports.forEach((report) => {
    let sameUrl = res.find((elem) => elem.url === report.url);
    if (sameUrl) {
      sameUrl.num_reports += 1;
      sameUrl.id.push(report.id);
      if (report.updated_at > sameUrl.updated_at)
        sameUrl.updated_at = report.updated_at;
      report.categories.forEach((category) => {
        if (!sameUrl?.categories.includes(category))
          sameUrl?.categories.push(category);
      });
    } else {
      res.push({
        ...report,
        id: [report.id],
        num_reports: 1,
        index: res.length,
      });
    }
  });
  return res;
};

const ReportTable: React.FC = () => {
  const [reportData, setReportData] = useState<AggregatedReport[]>([]);
  useEffect(() => {
    getReports().then((data) => setReportData(aggregateReports(data)));
  }, []);
  const headerStyles = "font-light text-left p-2";
  return (
    <table className="table-fixed w-full">
      <thead>
        <tr>
          <SortByHeader
            className={headerStyles}
            hook={[reportData, setReportData]}
            active
            fieldname="index"
          >
            Index
          </SortByHeader>
          <SortByHeader
            className={headerStyles}
            hook={[reportData, setReportData]}
            fieldname="url"
          >
            Url
          </SortByHeader>
          <SortByHeader
            className={headerStyles}
            hook={[reportData, setReportData]}
            fieldname="categories"
          >
            Reason
          </SortByHeader>
          <SortByHeader
            className={headerStyles}
            hook={[reportData, setReportData]}
            fieldname="num_reports"
          >
            Number of Reports
          </SortByHeader>
          <SortByHeader
            className={headerStyles}
            hook={[reportData, setReportData]}
            fieldname="updated_at"
          >
            Last Report
          </SortByHeader>
        </tr>
      </thead>
      <tbody>
        {reportData.map((report, i) => {
          const bg = i % 2 === 0 ? "bg-background-dark" : "bg-background-light";
          const styles = "font-light py-3 pl-3 pr-10 ";
          return (
            <tr key={i} className={styles + bg}>
              <td className={styles + bg}>{report.index}</td>
              <td
                className={`${styles + bg} overflow-ellipsis overflow-hidden`}
              >
                {
                  <a
                    className="underline text-cyan-500"
                    href={`https://cloudflare-ipfs.com/ipfs/${
                      report.url.split("/")[report.url.split("/").length - 1]
                    }`}
                  >
                    {report.url}
                  </a>
                }
              </td>
              <td className={styles + bg}>
                {report.categories.map(
                  (label, i) =>
                    label + (i !== report.categories.length - 1 ? ", " : "")
                )}
              </td>
              <td className={styles + bg}>{report.num_reports}</td>
              <td className={styles + bg}>{report.updated_at}</td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
};
export default ReportTable;
