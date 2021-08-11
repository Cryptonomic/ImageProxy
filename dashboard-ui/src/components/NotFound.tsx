import React from "react";

const NotFound = () => {
  return (
    <div className="w-full h-screen text-center flex-flex-col justify-center p-20 space-y-8">
      <div className="text-4xl">No Metrics Recieved</div>
      <div className="text-xl">
        Make sure Image Proxy is running and
        <code className="bg-gray-300 m-1 px-2 py-1">
          metrics_enabled = true
        </code>
        in your proxy.conf file
      </div>
    </div>
  );
};

export default NotFound;
