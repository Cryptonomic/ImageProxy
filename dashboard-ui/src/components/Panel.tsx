import React from "react";

const Panel: React.FC<{ className?: string }> = ({ children, className }) => {
  return (
    <div className={"w-full h-full p-0 bg-gray-300 items-center" + className}>
      {children}
    </div>
  );
};
export default Panel;
