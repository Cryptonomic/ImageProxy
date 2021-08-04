import React, { useRef, useEffect } from "react";
import {
  select,
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
  xAxisLabel?:string;
  yAxisLabel?:string;
  data: BarChartData[];
}
const BarChart: React.FC<Props> = ({
  width,
  height,
  data,
  title,
  className,
  xAxisLabel,
  yAxisLabel
}) => {
  const plotMarginLeft = 40;
  const plotMarginRight = -20;
  const plotMarginTop = 15;
  const plotMarginBottom = 20;
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
    const addXAxis = () => svg
        .append("g")
        .attr("transform", `translate(0,${plotHeight})`)
        .classed("x-axis", true)
        .call(xAxis)
        .append("text")
        .text(xAxisLabel ? xAxisLabel : "")
        .attr("transform", `translate(${plotWidth/2},${plotMarginBottom + 15})`)        
        .attr("fill", "black")        ;
    const addYAxis = () => svg
        .append("g")
        .classed("y-axis", true)
        .attr("transform", `translate(${plotMarginLeft},0)`)
        .call(yAxis)
        .append("text")
        .text(yAxisLabel? yAxisLabel : "")
        .attr("transform", `rotate(-90) translate(-${plotHeight/6}, -${20})`)
        .attr("fill", "black")
        .attr("x", 0)
        .attr("y", 0);
    if (!svg.select(".x-axis").node()) {
      addXAxis()
    }
    if (svg.select(".y-axis").node()) {
      svg.select(".y-axis").remove();
    } 
    addYAxis()
    svg.selectAll("path, line").style("stroke", "#d6d6d6");
  }, [data, plotHeight, plotWidth, xAxisLabel, yAxisLabel]);
  return (
    <div className={`flex flex-col ${className}`}>
      {title && <div className="text-center text-lg m-4">{title}</div>}
      <svg ref={svgRef} height={height} width={width}></svg>
    </div>
  );
};

export default BarChart;
