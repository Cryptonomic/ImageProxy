# NFT Image Proxy
Image Proxy for use with Tezos Wallets with support for content moderation.

# Quick Start
A docker compose file is provided with this repository. The following steps will allow you to setup your own proxy service:

1. Create an AWS account if you don't have one. Moderation features only work if you connect the proxy to AWS.
1. Create an IAM user with read permissions for Rekognition. See AWS (documentation)[https://docs.aws.amazon.com/rekognition/latest/dg/security_iam_service-with-iam.html] for more help on how to set this up.
1. Start the containers `docker-compose up --build`
1. Test if the service is up by visiting `http://localhost:3000/info` on your browser.

To run a custom configuration, uncomment the volume mount option for the proxy service in `docker-compose.yml`.

