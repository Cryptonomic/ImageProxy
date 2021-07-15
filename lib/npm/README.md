# NFT Image Proxy

`nft-image-proxy ` is a javascript/typescript api to interact with [NFT Image Proxy](https://github.com/Cryptonomic/ImageProxy) to allow for easy image filtering and moderation.

## Installation

`nft-image-proxy` is available via npm. Run `npm install nft-image-proxy`

## Usage

### Fetch

To fetch an image, use `proxyFetch`. This function takes in a server object, the url to fetch, the response type (either `ImageProxyDataType.Json` or `ImageProxyDataType.Raw`), and a `force` flag. If `force` is set to true, the image will be fetched regardless of if it contains explicit content. If it is set to `false`, then explicit images will be blocked and a JSON response will be returned which contains a list of moderation labels that the image was found to match with.

```typescript
import {
  proxyFetch,
  ImageProxyServer,
  ImageProxyDataType,
} from "nft-image-proxy";

const server: ImageProxyServer = {
  url: "https://imgproxy-prod.cryptonomic-infra.tech",
  version: "1.0.0",
  apikey: "myapikey",
};

proxyFetch(
  server,
  "https://upload.wikimedia.org/wikipedia/commons/8/84/Michelangelo%27s_David_2015.jpg",
  ImageProxyDataType.Json,
  false
).then((response) => console.log(response));
```

Some convenience functions are also available. `safeFetch` is equivilant to `proxyFetch` with the force parameter set to `false` and `unsafeFetch` is equivilant to `proxyFetch` with the `force` parameter set to `true`.

```typescript
import { safeFetch, unsafeFetch } from "nft-image-proxy";

safeFetch(server, url, ImageProxyDataType.Json).then(
  (response: FetchResponse | ImageProxyError) => console.log(response)
);

unsafeFetch(server, url, ImageProxyDataType.Json).then(
  (response: FetchResponse | ImageProxyError) => console.log(response)
);
```

### Describe

If the image proxy has seen an image previously, it can return moderation results from its cache before fetching. `describe` takes in an array of url strings and returns a JSON response containing a description of each image. The status of each image can be either `Blocked`, `Allowed`, or `NotSeen` (if the image hasn't been cached yet).

```typescript
import { describe } from "nft-image-proxy";

describe(server, [url1, url2, url3]).then(
  (response: DescribeResponse | ImageProxyError) => console.log(response)
);
```

### Report

If an explicit image were to make it past the content moderation provider, a user could also report the image to the proxy with suggested moderation labels.

```typescript
import { response } from "nft-image-proxy";

report(server, url, [ModerationLabel.Drugs, ModerationLabel.Alcohol]).then(
  (response: ReportResponse | ImageProxyError) => {
    console.log(response);
  }
);
```

### Describe Reports

All of the current reports submitted to the image proxy can be viewed using `describeReports`

```typescript
import { describeReports } from "nft-image-proxy";

describeReports(server).then(
  (response: DescribeReportsResponse | ImageProxyError) => console.log(response)
);
```

## Other References

For more information about the image proxy, the request and reponse formats, or how it works, view our public repo [here](https://github.com/Cryptonomic/ImageProxy).
