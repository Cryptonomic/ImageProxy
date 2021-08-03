import React, { useRef, useEffect } from "react";
import {
  select,
  line,
  scaleLinear,
  scaleTime,
  max,
  extent,
  axisBottom,
  axisLeft,
} from "d3";

interface Props {
  title?: string;
  width: number;
  height: number;
  data: [number, number][];
}

const LineGraph: React.FC<Props> = ({ width, height, title, data }) => {
  const plotMargin = 30;
  const plotWidth = width - plotMargin;
  const plotHeight = height - plotMargin;
  const svgRef = useRef(null);

  useEffect(() => {
    const y = scaleLinear()
      .domain([0, max(data, (d) => d[1])! + 10])
      .range([plotHeight, 0]);

    const x = scaleTime()
      .domain([Date.now() - 300000, Date.now()])
      .range([plotMargin, plotWidth]);

    const xAxis = axisBottom(x).tickSize(-plotHeight).tickPadding(10);
    const yAxis = axisLeft(y)
      .ticks(4, "s")
      .tickSize(-plotWidth)
      .tickPadding(10);
    const svg = select(svgRef.current);
    const l = line()
      .y((d) => y(d[1]))
      .x((d) => x(d[0]));
    svg
      .selectAll("path")
      .data([data.filter((d) => d[0] !== 0)])
      .join("path")
      .transition()
      .duration(200)
      .attr("d", (value) => l(value))
      .attr("fill", "none")
      .attr("stroke", "#FF7477");
    if (!svg.select(".x-axis").node() && data.length !== 0) {
      svg
        .append("g")
        .attr("transform", `translate(0,${plotHeight})`)
        .classed("x-axis", true)
        .call(xAxis);
    } else {
      svg.select(".x-axis").remove();
      svg
        .append("g")
        .attr("transform", `translate(0,${plotHeight})`)
        .classed("x-axis", true)
        .call(xAxis);
    }
    if (!svg.select(".y-axis").node() && data.length !== 0) {
      svg
        .append("g")
        .classed("y-axis", true)
        .attr("transform", `translate(${plotMargin},0)`)
        .call(yAxis);
    } else {
      svg.select(".y-axis").remove();
      svg
        .append("g")
        .attr("transform", `translate(${plotMargin},0)`)
        .classed("y-axis", true)
        .call(yAxis);
    }
    svg.selectAll(".y-axis path, line").style("stroke", "#d6d6d6");
  });
  return (
    <div className="flex flex-col">
      {title && <div className="text-center text-lg m-4">{title}</div>}
      <svg ref={svgRef} height={height} width={width}></svg>
    </div>
  );
};

export default LineGraph;
