import '../../bridge_generated.dart';

abstract class TypedWalletCore {
  Future<PinValidationResult> isValidPin(String pin);

  Future<void> register(String pin);

  Future<bool> isRegistered();

  Future<void> lockWallet();

  Future<WalletUnlockResult> unlockWallet(String pin);

  Stream<bool> get isLocked;

  Future<String> createPidIssuanceRedirectUri();

  Stream<ProcessUriEvent> processUri(Uri uri);

  Future<void> cancelPidIssuance();

  Stream<FlutterConfiguration> observeConfig();
}
