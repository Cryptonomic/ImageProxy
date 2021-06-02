import React, { Component, useEffect, useState } from "react";
import Dashboard from "./components/Dashboard";
import Navbar from "./components/Navbar";
import Panel from "./components/Panel";
import ReportTable from "./components/ReportTable";
import { getInfo, Info } from "./utils/ImageProxy";

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
        <Panel />
        <Panel>
          <ReportTable />
        </Panel>
      </Dashboard>
    </div>
  );
}

export default App;
