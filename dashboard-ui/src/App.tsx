import Dashboard from "./components/Dashboard";
import Navbar from "./components/Navbar";
import Panel from "./components/Panel";
import Metrics from "./components/Metrics";
import Info from "./components/Info";

function App() {
  return (
    <div className="flex flex-col h-full w-full bg-background">
      <Navbar />
      <Dashboard>
        <Panel name="Metrics">
          <Info />
          <Metrics />
        </Panel>
        <Panel name="User Reports" disabled />
        <Panel name="Moderation Reports" disabled />
        <Panel name="Configuration" disabled />
      </Dashboard>
    </div>
  );
}

export default App;
