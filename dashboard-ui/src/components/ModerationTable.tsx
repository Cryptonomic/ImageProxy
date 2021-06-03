import React, { useEffect, useState } from "react";
import { getModerationReports, ModerationInfo } from "../utils/ImageProxy";
import SortByHeader from "./SortByHeader";

const ModerationTable: React.FC = () => {
  const [reportData, setReportData] = useState<ModerationInfo[]>([]);
  useEffect(() => {
    getModerationReports().then((data) => setReportData(data));
  }, []);
  const headerStyles = "font-light text-left p-2";
  return (
    <table className="table-fixed w-full m-8">
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
            Categories
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
                    {" "}
                    {report.url}
                  </a>
                }
              </td>
              <td className={styles + bg}>
                {report.categories.map((label) => label + ", ")}
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
};
export default ModerationTable;
