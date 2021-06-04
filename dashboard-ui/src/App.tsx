import Dashboard from "./components/Dashboard";
import Navbar from "./components/Navbar";
import Panel from "./components/Panel";
import ReportTable from "./components/ReportTable";
import ModerationTable from "./components/ModerationTable";
import Metrics from "./components/Metrics";
import Configuration from "./components/Configuration";
import Info from "./components/Info";

function App() {
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
          <Info />
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

        <Panel>
          <Configuration />
        </Panel>
      </Dashboard>
    </div>
  );
}

export default App;
