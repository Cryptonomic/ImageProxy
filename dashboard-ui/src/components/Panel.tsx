import React from "react";

const Panel: React.FC<{ className?: string }> = ({ children, className }) => {
  return (
    <div className={"w-7/8 m-8 p-0 items-center" + className}>{children}</div>
  );
};
export default Panel;
