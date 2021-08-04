import React, { useRef, useEffect } from "react";
import {
  selectAll,
  select,
  line,
  scaleLinear,
  scaleTime,
  max,
  timeFormat,
  axisBottom,
  axisLeft,
} from "d3";

interface Props {
  title?: string;
  className?: string;
  width: number;
  height: number;
  xAxisLabel?: string;
  yAxisLabel?: string;  
  data: { label: string; color: string; coords: [number, number][] }[];
}

const LineGraph: React.FC<Props> = ({
  width,
  height,
  title,
  data,
  className,
  xAxisLabel,
  yAxisLabel
}) => {
  const plotMarginLeft = 40;
  const plotMarginRight = -20;
  const plotMarginTop = 20;
  const plotMarginBottom = 20;
  const plotWidth = width - plotMarginLeft - plotMarginRight;
  const plotHeight = height - plotMarginTop - plotMarginBottom;
  const svgRef = useRef(null);

  useEffect(() => {
    const svg = select(svgRef.current);
    let fullSet: [number, number][] = [];
    data.forEach(({ coords }) => (fullSet = fullSet.concat(coords)));

    const y = scaleLinear()
      .domain([0, max(fullSet, (d) => d[1])! * 1.5 + 10])
      .range([plotHeight, 0]);

    const x = scaleTime()
      .domain([Date.now() - 300000, Date.now()])
      .range([plotMarginLeft, plotWidth]);

    const xAxis = axisBottom(x)
      .tickSize(-plotHeight)
      .tickPadding(10)
      .tickFormat(((d: Date, i) =>
        i % 2 === 0 ? timeFormat("%H:%M")(d) : "") as (
        value: Date | { valueOf(): number },
        i: number
      ) => string);
    const yAxis = axisLeft(y)
      .ticks(4, "s")
      .tickSize(-plotWidth + plotMarginLeft)
      .tickPadding(10);
    data.forEach(({ label, color, coords }, i) => {
      const l = line()
        .y((d) => y(d[1]))
        .x((d) => x(d[0]));
      svg
        .selectAll(`.dataset-${i}`)
        .data([coords.filter((d) => d[0] !== 0)])
        .join("path")
        .transition()
        .duration(200)
        .attr("d", (value) => {
          return l(value);
        })
        .attr("fill", "none")
        .attr("stroke", color)
        .attr("stroke-width", 2)
        .attr("class", `dataset-${i}`);
      selectAll(`.label-${i}`)
        .append("rect")
        .attr("width", "20")
        .attr("height", "3")
        .attr("fill", color);
    });    
    const addXAxis = () => svg
      .append("g")
      .attr("transform", `translate(0,${plotHeight})`)
      .classed("x-axis", true)
      .call(xAxis)
      .append("text")
      .text(xAxisLabel ? xAxisLabel : "")
      .attr("transform", `translate(${plotWidth/2},${plotMarginBottom + 20})`)        
      .attr("fill", "black");
    const addYAxis = () => svg
      .append("g")
      .classed("y-axis", true)
      .attr("transform", `translate(${plotMarginLeft},0)`)
      .call(yAxis)      
      .append("text")
      .text(yAxisLabel? yAxisLabel : "")
      .attr("transform", `rotate(-90) translate(-${plotHeight/4}, -${30})`)
      .attr("fill", "black")
      .attr("x", 0)
      .attr("y", 0);
      
      if (svg.selectAll(".x-axis").node()) {
      svg.selectAll(".x-axis").remove();
    }
    addXAxis()
 
    if (svg.select(".y-axis").node()) {
      svg.select(".y-axis").remove();
    }
    addYAxis()
    
    svg.selectAll(".y-axis path, line").style("stroke", "#d6d6d6");
  });
  return (
    <div className={`flex flex-col ${className}`}>
      {title && <div className="text-center text-lg m-4">{title}</div>}
      <svg ref={svgRef} height={height} width={width}></svg>
      <div className="flex items-center justify-start w-full space-x-8">
        {data.map(({ label, color }, i) => (
          <div className="flex">
            <svg width={20} height={3} className={`m-2 label-${i}`} />{" "}
            <p className="text-xs">{label}</p>
          </div>
        ))}
      </div>
    </div>
  );
};

export default LineGraph;
