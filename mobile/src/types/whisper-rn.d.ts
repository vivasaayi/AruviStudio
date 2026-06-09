declare module "whisper.rn" {
  export type TranscribeResult = {
    result: string;
    language: string;
    segments: Array<{
      text: string;
      t0: number;
      t1: number;
    }>;
    isAborted: boolean;
  };

  export type TranscribeFileOptions = {
    language?: string;
    translate?: boolean;
    maxThreads?: number;
    nProcessors?: number;
    maxContext?: number;
    maxLen?: number;
    tokenTimestamps?: boolean;
    tdrzEnable?: boolean;
    wordThold?: number;
    offset?: number;
    duration?: number;
    temperature?: number;
    temperatureInc?: number;
    beamSize?: number;
    bestOf?: number;
    prompt?: string;
    onProgress?: (progress: number) => void;
    onNewSegments?: (result: unknown) => void;
  };

  export class WhisperContext {
    transcribe(filePathOrBase64: string | number, options?: TranscribeFileOptions): {
      stop: () => Promise<void>;
      promise: Promise<TranscribeResult>;
    };
    release(): Promise<void>;
  }

  export function initWhisper(options: {
    filePath: string | number;
    isBundleAsset?: boolean;
    useGpu?: boolean;
    useCoreMLIos?: boolean;
    useFlashAttn?: boolean;
  }): Promise<WhisperContext>;
}
