import {
  safeFetch,
  unsafeFetch,
  describe,
  describeReports,
  report,
  ReportRequest,
  ReportResponse,
  DescribeReportsRequest,
  DescribeReportsResponse,
  DescribeResponse,
  FetchResponse,
  ImageProxyDataType,
  ImageProxyError,
  ImageProxyServer,
  ModerationLabel,
} from "nft-image-proxy";

async function test() {
  const imageProxyServer: ImageProxyServer = {
    url: "http://localhost:3000",
    version: "1.0.0",
    apikey: "134472c4dd9118dbff1ed4e5fc7f1d056a0d690c9b6cc47c5c2453a011f57127",
  };
  const urls = [
    "https://upload.wikimedia.org/wikipedia/commons/1/1b/GreatBarrierReef-EO.JPG",
    "https://upload.wikimedia.org/wikipedia/commons/8/84/Michelangelo%27s_David_2015.jpg",
  ];

  const fetchResponse: FetchResponse | ImageProxyError = await unsafeFetch(
    imageProxyServer,
    "https://upload.wikimedia.org/wikipedia/commons/8/84/Michelangelo%27s_David_2015.jpg",
    ImageProxyDataType.Json
  );

  const describeResponse: DescribeResponse | ImageProxyError = await describe(
    imageProxyServer,
    urls
  );
  const reportResponse: ReportResponse | ImageProxyError = await report(
    imageProxyServer,
    urls[0],
    [ModerationLabel.Gambling]
  );
  const describeReportsResponse: DescribeReportsResponse | ImageProxyError =
    await describeReports(imageProxyServer);

  console.log(fetchResponse);
  console.log(describeResponse);
  console.log(reportResponse);
  console.log(describeReportsResponse);
  console.log(urls[0]);
}

test();
