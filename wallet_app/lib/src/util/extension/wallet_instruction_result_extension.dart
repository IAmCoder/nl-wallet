import 'package:wallet_core/core.dart';

import '../../domain/usecase/pin/check_pin_usecase.dart';
import 'wallet_instruction_error_extension.dart';

extension WalletInstructionResultExtension on WalletInstructionResult {
  CheckPinResult asCheckPinResult() {
    return map<CheckPinResult>(
      ok: (result) => CheckPinResultOk(),
      instructionError: (result) => result.error.asCheckPinResult(),
    );
  }
}
