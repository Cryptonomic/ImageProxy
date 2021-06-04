import { useEffect, useState } from "react";
import { getMetrics } from "../utils/ImageProxy";
import info from "../images/information.png";

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
  useEffect(() => {
    getMetrics().then((d) => setMetrics(d));
  }, []);
  const find = (name: string) =>
    metrics?.find((metric) => metric.name === name);
  console.log(metrics);
  return (
    <div className="w-full h-full flex flex-wrap content-start">
      <Block
        title="Uptime"
        value={find("uptime")?.metrics[0].value}
        units="Seconds"
        hint={find("uptime")?.help}
      />
      <Block
        title="Bytes Sent"
        value={find("bytes_sent")?.metrics[0].value}
        units="Bytes"
        hint={find("bytes_sent")?.help}
      />
      <Block
        title="Bytes Moderated"
        value={find("bytes_sent_mod")?.metrics[0].value}
        units="Bytes"
        hint={find("bytes_sent_mod")?.help}
      />
      <Block
        title="Bytes Fetched"
        value={find("bytes_fetched")?.metrics[0].value}
        units="Bytes"
        hint={find("bytes_fetched")?.help}
      />
      <Block
        title="Blocked Requests"
        value={find("docs_blocked")?.metrics[0].value}
        hint={find("docs_blocked")?.help}
      />
      <Block
        title="Fetched Requests"
        value={find("docs_fetched")?.metrics[0].value}
        hint={find("docs_fetched")?.help}
      />
      <Block
        title="Forced Requests"
        value={find("docs_forced")?.metrics[0].value}
        hint={find("docs_forced")?.help}
      />
      <Block
        title="API Hits"
        value={find("api_fetch")?.metrics[0].value}
        hint={find("api_fetch")?.help}
      />
      <Block
        title="Errors"
        value={find("errors")?.metrics[0].value}
        hint={find("errors")?.help}
      />
      <Block
        title="Cache Hits"
        value={find("cache_hit")?.metrics[0].value}
        hint={find("cache_hit")?.help}
      />
      <Block
        title="Cache Misses"
        value={find("cache_miss")?.metrics[0].value}
        hint={find("cache_miss")?.help}
      />
    </div>
  );
};

export default Metrics;
