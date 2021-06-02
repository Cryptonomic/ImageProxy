import React, { useState, FunctionComponent } from "react";

interface Props {
  names: string[];
}

const Dashboard: React.FC<Props> = ({ names, children }) => {
  const [ind, setInd] = useState(0);
  return (
    <div className="my-10 mx-12 h-full">
      <div className="flex flex-row">
        {names.map((name, i) => (
          <div
            key={i}
            className="mx-8 text-2xl font-light transform transition hover:scale-95 hover:opacity-50"
            onClick={() => setInd(i)}
          >
            {name}
            {
              <div
                className={
                  (i == ind ? "bg-orange" : "bg-transparent") +
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
