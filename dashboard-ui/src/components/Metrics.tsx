import { useEffect, useState } from "react";
import useInterval from "react-useinterval";
import {
  getMetrics,
  findMetric,
  findNestedMetric,
  findApiMetrics,
  findCacheMetrics,
  findApiResponseTimeMetrics,
} from "../utils/ImageProxy";
import info from "../images/information.png";
import LineGraph from "./LineGraph";
import BarChart from "./BarChart";

const Block: React.FC<{
  title: string;
  value: string;
  units?: string;
  hint?: string;
}> = ({ title, value, hint, units }) => {
  const [showToolTip, setShowing] = useState(false);

  return (
    <div className="relative h-44 w-1/6 min-w-min m-4 p-4 flex flex-col bg-background-dark rounded-lg transform transition hover:scale-95 ">
      {hint && (
        <img
          src={info}
          alt="info"
          className="w-4 absolute left-4 top-4 opacity-50"
          onMouseEnter={() => setShowing(true)}
          onMouseLeave={() => setShowing(false)}
        />
      )}
      <div
        className={`w-3/4 absolute left-12 top-6 transition-all duration-5000 ease-in-out opacity-${
          showToolTip ? "95" : "0"
        } bg-background-light rounded-lg p-4 text-center`}
      >
        {hint}
      </div>
      <p className="text-lg text-center">{title}</p>
      <div className="my-auto text-4xl text-center">
        {value} {units}
      </div>
    </div>
  );
};

const Metrics = () => {
  const [metrics, setMetrics] = useState<any[]>();
  const [totalRequests, setTotalRequests] = useState(0);
  const [reqsPerSec, setReqsPerSec] = useState(new Array(60).fill([0, 0]));
  const find = (name: string) => findMetric(metrics, name);
  const findNested = (name: string, labelName: string, label: string) =>
    findNestedMetric(metrics, name, labelName, label);
  const findCache = (name: string) => findCacheMetrics(metrics, name);
  const findApi = (name: string) => findApiMetrics(metrics, name);

  useInterval(() => {
    getMetrics().then((d) => {
      setMetrics(d);
      let curr =
        findApiMetrics(d, "img_proxy_fetch") +
        findApiMetrics(d, "img_proxy_describe") +
        findApiMetrics(d, "img_proxy_report") +
        findApiMetrics(d, "img_proxy_describe_report");

      console.log(reqsPerSec);
      if (totalRequests !== 0) {
        reqsPerSec.shift();
        reqsPerSec.push([Date.now(), (curr - totalRequests) / 5]);
        setReqsPerSec(reqsPerSec);
        console.log(curr, totalRequests);
      }
      setTotalRequests(curr);
    });
  }, 5000);

  console.log(metrics);
  return (
    <div className="w-full h-full flex flex-wrap content-start">
      <Block
        title="Uptime"
        value={(
          ((Date.now() / 1e3) as number) -
          find("process_start_time_seconds")?.metrics[0].value
        ).toFixed(3)}
        units="Seconds"
        hint={find("process_start_time_seconds")?.help}
      />
      <Block
        title="Cache Usage"
        value={(
          (findCache("mem_used_bytes")?.value /
            findCache("mem_total_bytes")?.value) *
          100
        )
          .toFixed(3)
          .toString()}
        units="%"
        hint={"Percentage of cache memory used"}
      />
      <Block
        title="Cache Mem"
        value={(findCache("mem_total_bytes")?.value / 1e6)
          .toFixed(3)
          .toString()}
        units="Mb"
        hint={"Total cache memory"}
      />

      <Block
        title="Cached Documents"
        value={findCache("items")?.value}
        units="Items"
        hint={"Number of items in cache"}
      />
      <Block
        title="Total Requests"
        value={totalRequests.toString()}
        hint={"Total number of requests made"}
      />
      <Block
        title="Fetched (Docs)"
        value={findNested("document", "status", "fetched")?.value || 0}
        hint={"Number of unforced fetches"}
      />
      <Block
        title="Forced (Docs)"
        value={findNested("document", "status", "forced")?.value || 0}
        hint={"Number of forced fetches"}
      />
      <Block
        title="Errors"
        value={find("errors")?.metrics[0].value}
        hint={find("errors")?.help}
      />

      <BarChart
        title={find("api_response_time")?.help}
        width={500}
        height={200}
        data={metrics ? findApiResponseTimeMetrics(metrics) : []}
      />
      <LineGraph
        title="Requests Per Second"
        width={500}
        height={200}
        data={reqsPerSec}
      />
    </div>
  );
};

export default Metrics;
