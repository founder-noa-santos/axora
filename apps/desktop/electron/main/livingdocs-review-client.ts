import { join } from "node:path";
import { promisify } from "node:util";

import * as grpc from "@grpc/grpc-js";
import * as protoLoader from "@grpc/proto-loader";

import {
  reviewDetailSchema,
  reviewQueueListSchema,
  reviewResolutionRequestSchema,
  reviewResolutionResponseSchema,
  type ReviewDetail,
  type ReviewQueueList,
  type ReviewResolutionRequest,
  type ReviewResolutionResponse,
  type ShellState,
} from "@/shared/contracts/desktop";

type RawGrpcClient = grpc.Client & {
  listPendingReviews(
    request: { workspaceRoot?: string; pageSize?: number; pageOffset?: number },
    callback: (error: grpc.ServiceError | null, response?: unknown) => void,
  ): void;
  getPendingReviewCount(
    request: { workspaceRoot?: string },
    callback: (error: grpc.ServiceError | null, response?: unknown) => void,
  ): void;
  getReviewDetail(
    request: { reviewId: string },
    callback: (error: grpc.ServiceError | null, response?: unknown) => void,
  ): void;
  submitResolution(
    request: {
      reviewId: string;
      choice: number;
      clientResolutionId: string;
      userNote?: string;
    },
    callback: (error: grpc.ServiceError | null, response?: unknown) => void,
  ): void;
};

type ProtoGrpcConstructor = new (
  address: string,
  credentials: grpc.ChannelCredentials,
) => RawGrpcClient;

const PROTO_PATH = join(
  __dirname,
  "..",
  "..",
  "..",
  "proto",
  "livingdocs",
  "v1",
  "review.proto",
);
const packageDefinition = protoLoader.loadSync(PROTO_PATH, {
  longs: Number,
  enums: String,
  defaults: true,
  oneofs: true,
});
const loaded = grpc.loadPackageDefinition(packageDefinition) as unknown as {
  livingdocs: {
    v1: {
      LivingDocsReviewService: ProtoGrpcConstructor;
    };
  };
};

const choiceToProto = {
  update_doc: 1,
  update_code: 2,
} as const;

export class LivingDocsReviewClient {
  private readonly endpoint: string;
  private readonly client: RawGrpcClient;

  constructor(endpoint = daemonEndpoint()) {
    this.endpoint = endpoint;
    this.client = new loaded.livingdocs.v1.LivingDocsReviewService(
      endpoint,
      grpc.credentials.createInsecure(),
    );
  }

  async getShellState(): Promise<ShellState> {
    try {
      await this.getPendingReviewCount("");
      return {
        rustBridge: {
          status: "connected",
          transport: "ipc",
          note: "Electron main brokers LivingDocs review gRPC calls to the daemon.",
        },
        daemon: {
          status: "online",
          endpoint: this.endpoint,
        },
      };
    } catch {
      return {
        rustBridge: {
          status: "connected",
          transport: "ipc",
          note: "Electron main brokers LivingDocs review gRPC calls to the daemon.",
        },
        daemon: {
          status: "offline",
          endpoint: this.endpoint,
        },
      };
    }
  }

  async getPendingReviewCount(workspaceRoot = ""): Promise<number> {
    const call = promisify(this.client.getPendingReviewCount.bind(this.client));
    const response = (await call({ workspaceRoot })) as { count: number };
    return response.count ?? 0;
  }

  async listPendingReviews(input?: {
    workspaceRoot?: string;
    pageSize?: number;
    pageOffset?: number;
  }): Promise<ReviewQueueList> {
    const call = promisify(this.client.listPendingReviews.bind(this.client));
    const response = (await call({
      workspaceRoot: input?.workspaceRoot ?? "",
      pageSize: input?.pageSize ?? 50,
      pageOffset: input?.pageOffset ?? 0,
    })) as {
      items?: Array<{
        reviewId: string;
        reportId: string;
        workspaceRoot: string;
        createdAtMs: number;
        confidenceScore: number;
        primaryDocPath: string;
        highestSeverity?: string;
        summary?: string;
      }>;
      totalPending?: number;
    };
    return reviewQueueListSchema.parse({
      items: response.items ?? [],
      totalPending: response.totalPending ?? 0,
    });
  }

  async getReviewDetail(reviewId: string): Promise<ReviewDetail> {
    const call = promisify(this.client.getReviewDetail.bind(this.client));
    const response = (await call({ reviewId })) as {
      header?: {
        reviewId: string;
        reportId: string;
        workspaceRoot: string;
        createdAtMs: number;
        confidenceScore: number;
        primaryDocPath: string;
        highestSeverity?: string;
        summary?: string;
      };
      flags?: Array<{
        fingerprint: string;
        domain: string;
        kind: string;
        severity: string;
        docPath: string;
        codePath?: string;
        symbolName?: string;
        ruleIds?: string[];
        message: string;
        expectedExcerpt: string;
        actualExcerpt: string;
      }>;
      breakdownJson: string;
      confidenceAuditActionId?: string;
    };
    if (!response.header) {
      throw new Error("missing review detail header");
    }
    return reviewDetailSchema.parse({
      ...response.header,
      flags: response.flags ?? [],
      breakdownJson: response.breakdownJson,
      confidenceAuditActionId: response.confidenceAuditActionId,
    });
  }

  async submitResolution(
    input: ReviewResolutionRequest,
  ): Promise<ReviewResolutionResponse> {
    const safeInput = reviewResolutionRequestSchema.parse(input);
    const call = promisify(this.client.submitResolution.bind(this.client));
    const response = (await call({
      reviewId: safeInput.reviewId,
      choice: choiceToProto[safeInput.choice],
      clientResolutionId: safeInput.clientResolutionId,
      userNote: safeInput.userNote,
    })) as {
      serverResolutionId: string;
      outcome: string;
      patchReceiptId?: string;
      toonChangelogEntryId?: string;
    };
    return reviewResolutionResponseSchema.parse({
      serverResolutionId: response.serverResolutionId,
      outcome: normalizeOutcome(response.outcome),
      patchReceiptId: response.patchReceiptId,
      toonChangelogEntryId: response.toonChangelogEntryId,
    });
  }
}

function daemonEndpoint(): string {
  const raw =
    process.env.OPENAKTA_MCP_ENDPOINT ??
    process.env.OPENAKTA_REVIEW_DAEMON_ENDPOINT ??
    "http://127.0.0.1:50061";
  return raw.replace(/^https?:\/\//, "");
}

function normalizeOutcome(value: string | undefined) {
  switch ((value ?? "").toLowerCase()) {
    case "ok":
    case "resolution_outcome_ok":
      return "ok";
    case "rejected":
    case "resolution_outcome_rejected":
      return "rejected";
    case "conflict":
    case "resolution_outcome_conflict":
      return "conflict";
    case "duplicate":
    case "resolution_outcome_duplicate":
      return "duplicate";
    case "internal_error":
    case "resolution_outcome_internal_error":
      return "internal_error";
    default:
      return "unspecified";
  }
}
