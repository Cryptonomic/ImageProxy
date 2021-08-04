import React, { useRef, useEffect, useState } from "react";
import {
  select,
  line,
  scaleLinear,
  scaleBand,
  max,
  axisBottom,
  axisLeft,
} from "d3";

interface BarChartData {
  name: string;
  value: number;
}

interface Props {
  title?: string;
  className?: string;
  width: number;
  height: number;
  data: BarChartData[];
}
const BarChart: React.FC<Props> = ({
  width,
  height,
  data,
  title,
  className,
}) => {
  const plotMarginLeft = 40;
  const plotMarginRight = -20;
  const plotMarginTop = 15;
  const plotMarginBottom = 15;
  const plotWidth = width - plotMarginLeft - plotMarginRight;
  const plotHeight = height - plotMarginTop - plotMarginBottom;
  const svgRef = useRef(null);

  useEffect(() => {
    const svg = select(svgRef.current);
    const y = scaleLinear()
      .domain([0, max(data, (d) => d.value)!])
      .range([plotHeight, 0]);
    const x = scaleBand()
      .domain(data.map((d) => d.name))
      .range([plotMarginLeft, plotWidth])
      .padding(0.05);

    const xAxis = axisBottom(x).tickSize(-plotHeight).tickPadding(10);
    const yAxis = axisLeft(y)
      .ticks(5, "s")
      .tickSize(-plotWidth + plotMarginLeft)
      .tickPadding(10);

    svg
      .selectAll("rect")
      .data(data)
      .join("rect")
      .transition()
      .duration(200)
      .attr("x", ({ name }) => x(name)!)
      .attr("y", ({ value }) => y(value))
      .attr("height", ({ value }) => plotHeight - y(value))
      .attr("width", x.bandwidth)
      .attr("fill", "#FF7477");
    if (!svg.select(".x-axis").node() && data.length !== 0) {
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
        .attr("transform", `translate(${plotMarginLeft},0)`)
        .call(yAxis);
    } else {
      svg.select(".y-axis").remove();
      svg
        .append("g")
        .attr("transform", `translate(${plotMarginLeft},0)`)
        .classed("y-axis", true)
        .call(yAxis);
    }
    svg.selectAll("path, line").style("stroke", "#d6d6d6");
  }, [data]);
  return (
    <div className={`flex flex-col ${className}`}>
      {title && <div className="text-center text-lg m-4">{title}</div>}
      <svg ref={svgRef} height={height} width={width}></svg>
    </div>
  );
};

export default BarChart;
