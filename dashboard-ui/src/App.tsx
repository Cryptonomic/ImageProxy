import Dashboard from "./components/Dashboard";
import Navbar from "./components/Navbar";
import Panel from "./components/Panel";
import Metrics from "./components/Metrics";
import Configuration from "./components/Configuration";
import Info from "./components/Info";

function App() {
  return (
    <div className="flex flex-col h-full w-full bg-background">
      <Navbar />
      <Dashboard names={["Metrics", "Configuration"]}>
        <Panel>
          <Info />
          <Metrics />
        </Panel>

        <Panel>
          <Configuration />
        </Panel>
      </Dashboard>
    </div>
  );
}

export default App;
