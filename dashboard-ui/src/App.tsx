import Dashboard from "./components/Dashboard";
import Navbar from "./components/Navbar";
import Panel from "./components/Panel";
import Metrics from "./components/Metrics";
import Info from "./components/Info";

function App() {
  return (
    <div className="flex flex-col h-full w-full bg-background">
      <Navbar />
      <Dashboard
        names={[
          "Metrics",
          "User Reports",
          "Moderation Reports",
          "Configuration",
        ]}
      >
        <Panel>
          <Info />
          <Metrics />
        </Panel>

        <Panel disabled />
        <Panel disabled />
        <Panel disabled />
      </Dashboard>
    </div>
  );
}

export default App;
