import React, { useRef, useEffect, useState } from "react";
import { select, line } from "d3";

const LineGraph = () => {
  const svgRef = useRef(null);
  const [data, setData] = useState<[number, number][]>([
    [10, 20],
    [20, 30],
    [30, 40],
    [40, 50],
    [50, 60],
  ]);

  useEffect(() => {
    const svg = select(svgRef.current);
    const myLine = line();
    svg
      .selectAll("path")
      .data([data])
      .join("path")
      .attr("d", (value) => myLine(value))
      .attr("fill", "none")
      .attr("stroke", "blue");
  });
  return (
    <>
      <svg ref={svgRef} />
    </>
  );
};

export default LineGraph;
