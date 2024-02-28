import {
  FetchRequest,
  DescribeRequest,
  ReportRequest,
  DescribeReportsRequest,
} from "./requests";
import {
  DescribeReportsResponse,
  DescribeResponse,
  FetchResponse,
  ReportResponse,
} from "./responses";

export interface ImageProxyServer {
  url: string;
  apikey: string;
  version: string;
}

export enum ModerationLabel {
  ExplicitNudity = "ExplicitNudity",
  Suggestive = "Suggestive",
  Violence = "Violence",
  VisuallyDisturbing = "VisuallyDisturbing",
  Rude = "Rude",
  Drugs = "Drugs",
  Tobacco = "Tobacco",
  Alcohol = "Alcohol",
  Gambling = "Gambling",
  Hate = "Hate",
  ExplicitContent = "ExplicitContent",
  DrugsAndTobacco = "DrugsAndTobacco",
  Unknown = "Unknown",
}

export enum ModerationStatus {
  Allowed,
  Blocked,
}

export enum DocumentStatus {
  NeverSeen,
  Allowed,
  Blocked,
}

export type ImageProxyRequest =
  | FetchRequest
  | DescribeRequest
  | ReportRequest
  | DescribeReportsRequest;

export type ImageProxyResponse =
  | FetchResponse
  | DescribeResponse
  | ReportResponse
  | DescribeReportsResponse;
