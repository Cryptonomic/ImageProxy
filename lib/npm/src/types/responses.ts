import { ModerationLabel, ModerationStatus, DocumentStatus } from "./common";

export enum RpcStatus {
  Ok,
  Err,
}

interface ResponseHeader {
  jsonrpc: string;
  rpc_status: RpcStatus;
}

interface FetchBody {
  result: {
    moderation_status: ModerationStatus;
    categories: ModerationLabel[];
    data: string;
  };
}

interface DescribeBody {
  result: {
    url: string;
    status: DocumentStatus;
    categories: ModerationLabel[];
    provider: String;
  }[];
}

interface ReportBody {
  result: { url: string; id: string };
}

interface DescribeReportsBody {
  result: {
    url: string;
    categories: ModerationLabel[];
    id: string;
    updated_at: string;
  }[];
}

interface ErrorBody {
  error: {
    code: number;
    reason: string;
    request_id: string;
  };
}

export type FetchResponse = (ResponseHeader & FetchBody) | string;
export type DescribeResponse = ResponseHeader & DescribeBody;
export type ReportResponse = ResponseHeader & ReportBody;
export type DescribeReportsResponse = ResponseHeader & DescribeReportsBody;
export type ImageProxyError = ResponseHeader & ErrorBody;
