import { useEffect, useState } from "react";
import Dashboard from "./components/Dashboard";
import Navbar from "./components/Navbar";
import Panel from "./components/Panel";
import ReportTable from "./components/ReportTable";
import ModerationTable from "./components/ModerationTable";
import { getInfo, Info } from "./utils/ImageProxy";
import Metrics from "./components/Metrics";

function App() {
  const [info, setInfo] = useState<Info>();
  useEffect(() => {
    getInfo().then((i) => setInfo(i));
  }, []);
  return (
    <div className="flex flex-col h-full w-full bg-background">
      <Navbar />
      <Dashboard
        names={[
          "Info",
          "Metrics",
          "User Reports",
          "Moderation Reports",
          "Configuration",
        ]}
      >
        <Panel>
          <div className="my-12 m-8">
            <div>Package Version: {info?.package_version} </div>
            <div>Git Version: {info?.git_version} </div>
          </div>
        </Panel>

        <Panel>
          <Metrics />
        </Panel>

        <Panel>
          <ReportTable />
        </Panel>

        <Panel>
          <ModerationTable />
        </Panel>

        <Panel />
      </Dashboard>
    </div>
  );
}

export default App;
