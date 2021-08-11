import { useState, useEffect } from "react";
import useInterval from "react-useinterval";
import {
  getMetrics,
  findMetric,
  findNestedMetric,
  findApiMetrics,
  findCacheMetrics,
  findApiResponseTimeMetrics,
  secondsToHMS,
} from "../utils/ImageProxy";
import NotFound from "./NotFound";
import info from "../images/information.png";
import LineGraph from "./LineGraph";
import BarChart from "./BarChart";

const Block: React.FC<{
  title: string;
  className?: string;
  value?: string | number;
  units?: { unit: string; threshold: number }[] | string;
  hint?: string;
}> = ({ title, value, hint, units, children, className }) => {
  const [showToolTip, setShowing] = useState(false);
  const chosenUnit =
    typeof units == "string"
      ? { unit: units, threshold: 1 }
      : (units as { unit: string; threshold: number }[])
          ?.sort((a, b) => b.threshold - a.threshold)
          .find(({ threshold }) => threshold <= (value ? value : 0)) || {
          unit: "",
          threshold: 1,
        };
  const numericValue =
    (value as number) / (chosenUnit ? chosenUnit.threshold : 1);
  const valueToDisplay =
    typeof value == "number"
      ? numericValue === Math.floor(numericValue)
        ? numericValue
        : numericValue.toFixed(3)
      : value;

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
        {valueToDisplay} {chosenUnit?.unit} {children}
      </div>
    </div>
  );
};

const Metrics = () => {
  const [metrics, setMetrics] = useState<any[] | undefined>();
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
  const standardUnits = [
    { unit: "K", threshold: 1e3 },
    { unit: "Mil", threshold: 1e6 },
    { unit: "Bil", threshold: 1e9 },
  ];
  const digitalUnits = [
    { unit: "B", threshold: 1 },
    { unit: "Kb", threshold: 1e3 },
    { unit: "Mb", threshold: 1e6 },
    { unit: "Gb", threshold: 1e9 },
  ];

  const update = () =>
    getMetrics().then((d) => {
      if (d === 400) {
        setMetrics(undefined);
      } else {
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
      }
    });
  /* update(); */
  useEffect(() => {
    update();
  }, []);
  useInterval(() => {
    update();
  }, 5000);
  return (
    <div className="w-full h-full flex flex-wrap content-start justify-center">
      {metrics === undefined && <NotFound />}
      {metrics && (
        <>
          <div className="flex flex-wrap justify-center">
            <Block
              title="Uptime"
              value={secondsToHMS(
                find("process_start_time_seconds")
                  ? ((Date.now() / 1e3) as number) -
                      find("process_start_time_seconds")?.metrics[0].value
                  : 0
              )}
              hint={
                find("process_start_time_seconds")?.help ||
                "Uptime metric is only available on linux"
              }
            />

            <Block
              title="Total CPU Time"
              value={secondsToHMS(
                find("process_cpu_seconds_total")?.metrics[0].value || 0
              )}
              hint={
                find("process_cpu_seconds_total")?.help ||
                "Total CPU time metric is only available on linux"
              }
            />

            <Block
              title="Fetched Images"
              value={
                parseInt(findNested("document", "status", "fetched")?.value) ||
                0
              }
              units={standardUnits}
              hint={"Number of unforced fetches"}
            />
            <Block
              title="Forced Images"
              value={
                parseInt(findNested("document", "status", "forced")?.value) || 0
              }
              hint={"Number of forced fetches"}
            />
            <Block
              title="Moderation Requests"
              value={
                parseInt(
                  findNested("moderation", "metric", "requests")?.value
                ) || 0
              }
              units={standardUnits}
              hint={"Number of unforced fetches"}
            />
            <Block
              title="Total Requests"
              value={totalRequests}
              units={standardUnits}
              hint={"Total number of requests made"}
            />

            <Block
              title="Errors"
              value={parseInt(find("errors")?.metrics[0].value) || 0}
              units={standardUnits}
              hint={find("errors")?.help}
            />

            <Block
              title="Cache Usage"
              value={
                findCache("mem_used_bytes")?.value
                  ? (
                      (findCache("mem_used_bytes")?.value /
                        findCache("mem_total_bytes")?.value) *
                      100
                    ).toFixed(3)
                  : "No Cache"
              }
              units={findCache("mem_used_bytes") ? "%" : ""}
              hint={"Percentage of cache memory used"}
            />
            <Block
              title="Cache Mem"
              value={
                parseInt(findCache("mem_total_bytes")?.value) || "No Cache"
              }
              units={digitalUnits}
              hint={"Total cache memory"}
            />

            <Block
              title="Cached Documents"
              value={parseInt(findCache("items")?.value) || 0}
              units={standardUnits}
              hint={"Number of items in cache"}
            />
            <Block
              title="Cache Hits"
              value={parseInt(findCache("hits")?.value) || 0}
              units={standardUnits}
              hint={
                "Number of times an item was found and retrieved from cache"
              }
            />

            <Block
              title="Cache Misses"
              value={parseInt(findCache("misses")?.value) || 0}
              units={standardUnits}
              hint={"Number of times an item was not found in cache"}
            />
            <Block
              title="Cache Evictions"
              value={parseInt(findCache("evictions")?.value) || 0}
              units={standardUnits}
              hint={
                "Number of times an item was removed from cache to make space for other items"
              }
            />
            <Block
              title="Virtual Memory"
              value={
                parseInt(
                  find("process_virtual_memory_bytes")?.metrics[0].value
                ) || "Not available"
              }
              hint={
                find("process_virtual_memory_bytes")?.help ||
                "Virtual memory monitoring is only available on linux"
              }
              units={digitalUnits}
            />
          </div>
          <Block
            title="Api Response Time in Milliseconds"
            hint="Bar chart showing query responses grouped by response time"
            className="h-auto"
          >
            <BarChart
              width={800}
              height={200}
              xAxisLabel="Response Time (ms)"
              yAxisLabel="Number of Responses"
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
              yAxisLabel="Requests Per Second"
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
              yAxisLabel="Process Memory Usage"
              data={[
                {
                  label: "bytes used",
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
            <LineGraph
              width={800}
              height={200}
              yAxisLabel="Traffic Per Second"
              data={[
                {
                  label: "bytes fetched per second",
                  color: "#FF7477",
                  coords: trafficFetchedPerSec,
                },
                {
                  label: "bytes served per second",
                  color: "#47f9ff",
                  coords: trafficServedPerSec,
                },
              ]}
              className="m-4"
            />
          </Block>
        </>
      )}
    </div>
  );
};

export default Metrics;
