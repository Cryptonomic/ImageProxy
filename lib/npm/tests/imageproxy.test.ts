import {
  proxyFetch,
  safeFetch,
  unsafeFetch,
  describe as imageProxyDescribe,
  report,
  describeReports,
  ImageProxyDataType,
  FetchResponse,
  ModerationLabel,
} from "../src";

import { expect } from "chai";
import * as server from "./config.json";

const safeUrls = [
  "https://upload.wikimedia.org/wikipedia/commons/1/1b/GreatBarrierReef-EO.JPG",
];
const unsafeUrls = [
  "https://upload.wikimedia.org/wikipedia/commons/8/84/Michelangelo%27s_David_2015.jpg",
];

describe("Image proxy tests", () => {
  it("should allow a safe image to be fetched as JSON", async () => {
    const safeFetchResult = await safeFetch(
      server,
      safeUrls[0],
      ImageProxyDataType.Json
    );
    const unsafeFetchResult = await safeFetch(
      server,
      safeUrls[0],
      ImageProxyDataType.Json
    );
    const proxyFetchResult = await proxyFetch(
      server,
      safeUrls[0],
      ImageProxyDataType.Json,
      false
    );

    expect(safeFetchResult.result)
      .to.be.an("object")
      .to.have.property("moderation_status")
      .to.equal("Allowed");
    expect(unsafeFetchResult.result)
      .to.be.an("object")
      .to.have.property("moderation_status")
      .to.equal("Allowed");
    expect(proxyFetchResult.result)
      .to.be.an("object")
      .to.have.property("moderation_status")
      .to.equal("Allowed");
  });
  it("should allow a safe image to be fetched as raw bytes", async () => {
    const safeFetchResult = await unsafeFetch(
      server,
      safeUrls[0],
      ImageProxyDataType.Raw
    );
    const unsafeFetchResult = await unsafeFetch(
      server,
      safeUrls[0],
      ImageProxyDataType.Raw
    );
    const proxyFetchResult = await proxyFetch(
      server,
      safeUrls[0],
      ImageProxyDataType.Raw,
      false
    );

    expect(safeFetchResult).to.be.a("string");
    expect(unsafeFetchResult).to.be.a("string");
    expect(proxyFetchResult).to.be.a("string");
  });

  it("should block an unsafe image to be fetched as JSON when not forced", async () => {
    const safeFetchResult = await safeFetch(
      server,
      unsafeUrls[0],
      ImageProxyDataType.Json
    );
    const proxyFetchResult = await proxyFetch(
      server,
      unsafeUrls[0],
      ImageProxyDataType.Json,
      false
    );

    expect(safeFetchResult.result)
      .to.be.an("object")
      .to.have.property("moderation_status")
      .to.equal("Blocked");
    expect(proxyFetchResult.result)
      .to.be.an("object")
      .to.have.property("moderation_status")
      .to.equal("Blocked");
  });

  it("should block an unsafe image to be fetched as raw bytes when not forced", async () => {
    const safeFetchResult = await safeFetch(
      server,
      unsafeUrls[0],
      ImageProxyDataType.Raw
    );
    const proxyFetchResult = await proxyFetch(
      server,
      unsafeUrls[0],
      ImageProxyDataType.Raw,
      false
    );

    expect(safeFetchResult.result)
      .to.be.an("object")
      .to.have.property("moderation_status")
      .to.equal("Blocked");
    expect(proxyFetchResult.result)
      .to.be.an("object")
      .to.have.property("moderation_status")
      .to.equal("Blocked");
  });

  it("should allow an unsafe image to be fetched as JSON when forced", async () => {
    const unsafeFetchResult = await unsafeFetch(
      server,
      unsafeUrls[0],
      ImageProxyDataType.Json
    );
    const proxyFetchResult = await proxyFetch(
      server,
      unsafeUrls[0],
      ImageProxyDataType.Json,
      true
    );

    expect(unsafeFetchResult.result)
      .to.be.an("object")
      .to.have.property("moderation_status")
      .to.equal("Allowed");
    expect(proxyFetchResult.result)
      .to.be.an("object")
      .to.have.property("moderation_status")
      .to.equal("Allowed");
  });

  it("should allow an unsafe image to be fetched as raw bytes when forced", async () => {
    const unsafeFetchResult = await unsafeFetch(
      server,
      unsafeUrls[0],
      ImageProxyDataType.Raw
    );
    const proxyFetchResult = await proxyFetch(
      server,
      unsafeUrls[0],
      ImageProxyDataType.Raw,
      true
    );

    expect(unsafeFetchResult).to.be.a("string");
    expect(proxyFetchResult).to.be.a("string");
  });

  it("should describe urls correctly", async () => {
    const safeDescribeResponse = await imageProxyDescribe(server, safeUrls);
    const unsafeDescribeResponse = await imageProxyDescribe(server, unsafeUrls);
    expect(safeDescribeResponse.result[0]).to.exist;
    expect(unsafeDescribeResponse.result[0]).to.exist;
    expect(safeDescribeResponse.result[0].status).to.equal("Allowed");
    expect(unsafeDescribeResponse.result[0].status).to.equal("Blocked");
  });

  it("should report urls correctly", async () => {
    const reportResponse = await report(server, safeUrls[0], [
      ModerationLabel.Drugs,
    ]);

    expect(reportResponse.result)
      .to.be.an("object")
      .to.have.property("url")
      .to.equal(safeUrls[0]);
  });

  it("should describe reports correctly", async () => {
    const describeReportResponse = await describeReports(server);
    expect(describeReportResponse.result).to.exist;
    expect(
      describeReportResponse.result.find(
        (report) =>
          report.url == safeUrls[0] &&
          report.categories.length == 1 &&
          report.categories.includes(ModerationLabel.Drugs)
      )
    ).to.exist;
  });
});
