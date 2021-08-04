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
  className?: string;
  value?: string;
  units?: string;
  hint?: string;
}> = ({ title, value, hint, units, children, className }) => {
  const [showToolTip, setShowing] = useState(false);

  return (
    <div
      className={`relative h-44 w-1/6 min-w-min m-4 p-4 flex flex-col bg-background-dark rounded-lg transform transition duration-75	 hover:scale-105 ${className}`}
    >
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
        {value} {units} {children}
      </div>
    </div>
  );
};

const Metrics = () => {
  const [metrics, setMetrics] = useState<any[]>();
  const [totalRequests, setTotalRequests] = useState(0);
  const [reqsPerSec, setReqsPerSec] = useState<[number, number][]>(
    new Array(60).fill([0, 0])
  );
  const [memTimeSeries, setMemTimeSeries] = useState(
    new Array(60).fill([0, 0])
  );
  const [trafficFetched, setTrafficFetched] = useState(0);
  const [trafficServed, setTrafficServed] = useState(0);
  const [trafficFetchedPerSec, setTrafficFetchedPerSec] = useState<
    [number, number][]
  >(new Array(60).fill([0, 0]));
  const [trafficServedPerSec, setTrafficServedPerSec] = useState<
    [number, number][]
  >(new Array(60).fill([0, 0]));
  const find = (name: string) => findMetric(metrics, name);
  const findNested = (name: string, labelName: string, label: string) =>
    findNestedMetric(metrics, name, labelName, label);
  const findCache = (name: string) => findCacheMetrics(metrics, name);

  useInterval(() => {
    getMetrics().then((d) => {
      setMetrics(d);
      const currSumRequests =
        findApiMetrics(d, "img_proxy_fetch") +
        findApiMetrics(d, "img_proxy_describe") +
        findApiMetrics(d, "img_proxy_report") +
        findApiMetrics(d, "img_proxy_describe_report");
      const currResMem = find("process_resident_memory_bytes")?.metrics[0]
        .value;
      const currTrafficFetched = findNested(
        "traffic",
        "metric",
        "fetched"
      )?.value;
      const currTrafficServed = findNested(
        "traffic",
        "metric",
        "served"
      )?.value;
      console.log(currTrafficFetched, trafficFetched);
      if (trafficFetched && trafficFetched !== 0) {
        trafficFetchedPerSec.shift();
        trafficFetchedPerSec.push([
          Date.now(),
          (currTrafficFetched - trafficFetched) / 5,
        ]);
        setTrafficFetchedPerSec(trafficFetchedPerSec);
      }
      if (trafficServed && trafficServed !== 0) {
        trafficServedPerSec.shift();
        trafficServedPerSec.push([
          Date.now(),
          (currTrafficServed - trafficServed) / 5,
        ]);
        setTrafficServedPerSec(trafficServedPerSec);
      }
      if (totalRequests !== 0) {
        reqsPerSec.shift();
        reqsPerSec.push([Date.now(), (currSumRequests - totalRequests) / 5]);
        setReqsPerSec(reqsPerSec);
      }
      if (currResMem) {
        memTimeSeries.shift();
        memTimeSeries.push([Date.now(), parseInt(currResMem)]);
      }
      setTrafficFetched(currTrafficFetched);
      setTrafficServed(currTrafficServed);
      setMemTimeSeries(memTimeSeries);
      setTotalRequests(currSumRequests);
    });
  }, 5000);

  console.log(metrics);
  return (
    <div className="w-full h-full flex flex-wrap content-start justify-center">
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

      <Block
        title="Virtual Memory"
        value={find("process_virtual_memory_bytes")?.metrics[0].value}
        hint={find("process_virtual_memory_bytes")?.help}
        units="Bytes"
      />
      <Block
        title="Total CPU Time"
        value={find("process_cpu_seconds_total")?.metrics[0].value}
        hint={find("process_cpu_seconds_total")?.help}
        units="Seconds"
      />
      <Block
        title="Api Response Time in Milliseconds"
        hint="Bar chart showing query responses grouped by reesponse time"
        className="h-auto"
      >
        <BarChart
          width={800}
          height={200}
          data={metrics ? findApiResponseTimeMetrics(metrics) : []}
          className="m-4"
        />
      </Block>
      <Block
        title="Requests Per Second"
        hint="Number of requests received per second for the last 5 minutes"
        className="h-auto"
      >
        <LineGraph
          width={800}
          height={200}
          data={[
            {
              label: "requests per second",
              color: "#FF7477",
              coords: reqsPerSec,
            },
          ]}
          className="m-4"
        />
      </Block>
      <Block
        title="Process Memory"
        hint="Resident process memory being used over time"
        className="h-auto"
      >
        <LineGraph
          width={800}
          height={200}
          data={[
            {
              label: "Mb Per Second",
              color: "#FF7477",
              coords: memTimeSeries,
            },
          ]}
          className="m-4"
        />
      </Block>
      <Block
        title="Traffic"
        hint="Amount of data sent/recieved per second"
        className="h-auto"
      >
        {console.log(trafficFetchedPerSec)}
        <LineGraph
          width={800}
          height={200}
          data={[
            {
              label: "data fetched per second",
              color: "#FF7477",
              coords: trafficFetchedPerSec,
            },
            {
              label: "data served per second",
              color: "#47f9ff",
              coords: trafficServedPerSec,
            },
          ]}
          className="m-4"
        />
      </Block>
    </div>
  );
};

export default Metrics;
