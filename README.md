# NFT Image Proxy
Image Proxy for use with Tezos Wallets with support for content moderation.

The proxy supports the following features:

1. Fetching images from either `HTTP` or `IPFS` urls. 
1. The following image formats are supported: `bmp`, `jpg`, `png`, `tiff`, `gif`.
1. Automatic content moderation by hooking with a moderation provider.
   1. Automatic format conversion to a format supported by the moderation provider.
   1. Automatic image resizing to support file size limits set by the moderation provider.
1. User reporting of content that slips by the automatic moderation.

The proxy currently support AWS Rekognition as its moderation provider. There are plans for introducing other providers such as Azure in the future.

# Quick Start
A docker compose file is provided with this repository. The following steps will allow you to setup your own proxy service:

1. Create an AWS account if you don't have one. Moderation features only work if you connect the proxy to AWS.
1. Create an IAM user with read permissions for Rekognition. See AWS (documentation)[https://docs.aws.amazon.com/rekognition/latest/dg/security_iam_service-with-iam.html] for more help on how to set this up.
1. Create and download a AWS Access Keys from the IAM for the user you created in the previous step. Keep this information safe.
1. Create and `.env` file in the project root with the following contents:
    ```
    AWS_ACCESS_KEY_ID=<YOUR AWS ACCESS KEY ID>
    AWS_SECRET_ACCESS_KEY=<YOUR AWS ACCESS KEY>
    ```
1. Modify the configuration files `proxy.conf` to suit your needs. Pay attention to the AWS region setting. Ideally this should be set to the same region as where your proxy server is hosted. See AWS documentation on all possible values.
1. Start the containers `docker-compose up`
1. Test if the service is up by visiting `http://localhost:3000/info` on your browser.

Note that CORS supported is not baked in yet, but is upcoming very soon in a future PR. For the time being we recommend that you place a nginx reverse proxy in front of the service to handle CORS.

