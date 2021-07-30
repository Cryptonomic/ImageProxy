import React, { ReactElement, ReactFragment, useState } from "react";
import Panel from "./Panel";

interface Props {
  names: string[];
}

const Dashboard: React.FC<Props> = ({ names, children }) => {
  const [ind, setInd] = useState(0);
  const isDisabled = (i: number) =>
    (React.Children.toArray(children)[i] as ReactElement).props.disabled;
  return (
    <div className="my-10 mx-12 h-full">
      <div className="flex flex-row">
        {names.map((name, i) => (
          <div
            key={i}
            title={isDisabled(i) && "Coming soon"}
            className={`mx-8 text-2xl font-light transform transition ${
              isDisabled(i)
                ? "scale-95 opacity-50"
                : "hover:scale-95 hover:opacity-50"
            } `}
            onClick={() => (isDisabled(i) ? setInd(0) : setInd(ind))}
          >
            {name}
            {
              <div
                className={
                  (i === ind ? "bg-orange" : "bg-transparent") +
                  " h-1 w-full my-2"
                }
              />
            }
          </div>
        ))}
      </div>
      {React.Children.toArray(children)[ind]}
    </div>
  );
};

export default Dashboard;
