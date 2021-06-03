import React, { useState } from "react";
import chevron from "../images/chevron.svg";

interface HeaderProps {
  hook: [any[], React.Dispatch<React.SetStateAction<any[]>>];
  fieldname: string;
  className?: string;
  active?: boolean;
}

const SortByHeader: React.FC<HeaderProps> = ({
  hook,
  fieldname,
  className,
  children,
  active,
}) => {
  const [data, update] = hook;
  const [isAsc, setIsAsc] = useState(false);
  const [hov, setHov] = useState(false);
  const [isActive, setIsActive] = useState(active || false);

  const asc = (firstElem: any, secondElem: any) =>
    firstElem[fieldname] <= secondElem[fieldname] ? -1 : 1;
  const desc = (firstElem: any, secondElem: any) =>
    firstElem[fieldname] > secondElem[fieldname] ? -1 : 1;

  return (
    <th
      className={className}
      onMouseEnter={() => setHov(true)}
      onMouseLeave={() => setHov(false)}
    >
      <div className="flex flex-row">
        <div className="mr-4">{children}</div>
        <div className="flex flex-col justify-center">
          <img
            src={chevron}
            className={`w-4 h-4 opacity-${
              hov || isActive ? 50 : 0
            } hover:opacity-100 transform transition rotate-${isAsc ? 180 : 0}`}
            alt="arrow"
            onClick={() => {
              setIsAsc(!isAsc);
              update([...data].sort(isAsc ? asc : desc));
              setIsActive(true);
            }}
          />
        </div>
      </div>
    </th>
  );
};

export default SortByHeader;
