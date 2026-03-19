import { AccountService } from "./account-service";

export { AccountService } from "./account-service";
export type { AuthSnapshot, SavedProfile } from "./account-service";
export {
  AccountAlreadyExistsError,
  AccountNotFoundError,
  AuthFileInvalidError,
  AuthFileMissingError,
  CodexAuthError,
  InvalidAccountNameError,
  NoAccountsSavedError,
  PromptCancelledError,
} from "./errors";

export const accountService = new AccountService();
