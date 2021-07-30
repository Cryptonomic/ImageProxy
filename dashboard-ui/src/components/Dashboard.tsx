import React, { ReactElement, ReactFragment, useState } from "react";
import Panel from "./Panel";

const Dashboard: React.FC = ({ children }) => {
  const [ind, setInd] = useState(0);
  return (
    <div className="my-10 mx-12 h-full">
      <div className="flex flex-row">
        {React.Children.map(children, (child, i) => {
          const c = child as ReactElement;
          return (
            <div
              key={i}
              title={c.props.disabled && "Coming soon"}
              className={`mx-8 text-2xl font-light transform transition ${
                c.props.disabled
                  ? "scale-95 opacity-50"
                  : "hover:scale-95 hover:opacity-50"
              } `}
              onClick={() => (c.props.disabled ? setInd(0) : setInd(ind))}
            >
              {c.props.name}
              {
                <div
                  className={
                    (i === ind ? "bg-orange" : "bg-transparent") +
                    " h-1 w-full my-2"
                  }
                />
              }
            </div>
          );
        })}
      </div>
      {React.Children.toArray(children)[ind]}
    </div>
  );
};

export default Dashboard;
