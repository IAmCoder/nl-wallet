import 'dart:convert';

import 'package:flutter_rust_bridge/flutter_rust_bridge.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mockito/mockito.dart';
import 'package:wallet/src/wallet_core/error/core_error.dart';
import 'package:wallet/src/wallet_core/error/core_error_mapper.dart';
import 'package:wallet/src/wallet_core/error/flutter_api_error.dart';
import 'package:wallet/src/wallet_core/typed/typed_wallet_core.dart';
import 'package:wallet_core/core.dart';

import '../../mocks/wallet_mocks.dart';

const samplePin = '112233';

void main() {
  late WalletCore core;
  late TypedWalletCore typedWalletCore;

  setUp(() {
    core = Mocks.create();
    typedWalletCore = TypedWalletCore(core, CoreErrorMapper()); //FIXME: Mock mapper

    /// Setup default initialization mock
    when(core.isInitialized()).thenAnswer((realInvocation) async => false);
    when(core.init()).thenAnswer((realInvocation) async => true);
  });

  group('isValidPin', () {
    test('pin validation is passed on to core', () async {
      when(core.isValidPin(pin: samplePin)).thenAnswer((realInvocation) async => PinValidationResult.Ok);
      final result = await typedWalletCore.isValidPin(samplePin);
      expect(result, PinValidationResult.Ok);
      verify(core.isValidPin(pin: samplePin)).called(1);
    });
  });

  group('register', () {
    test('register is passed on to core', () async {
      await typedWalletCore.register(samplePin);
      verify(core.register(pin: samplePin)).called(1);
    });
  });

  group('isRegistered', () {
    test('registration check is passed on to core', () async {
      await typedWalletCore.isRegistered();
      verify(core.hasRegistration()).called(1);
    });
  });

  group('lockWallet', () {
    test('lock wallet is passed on to core', () async {
      await typedWalletCore.lockWallet();
      verify(core.lockWallet()).called(1);
    });
  });

  group('unlockWallet', () {
    test('unlock wallet is passed on to core', () async {
      await typedWalletCore.unlockWallet(samplePin);
      verify(core.unlockWallet(pin: samplePin)).called(1);
    });
  });

  group('isLocked', () {
    test('locked state is fetched through core by setting the lock stream', () async {
      // Verify we don't observe the stream pre-emptively
      verifyNever(core.setLockStream());
      // But make sure we do call into the core once we check the isLocked stream
      await typedWalletCore.isLocked.first;
      verify(core.setLockStream()).called(1);
    });
  });

  group('createdPidIssuanceUri', () {
    test('create pid issuance redirect uri is passed on to core', () async {
      await typedWalletCore.createPidIssuanceRedirectUri();
      verify(core.createPidIssuanceRedirectUri()).called(1);
    });
  });

  group('identifyUri', () {
    test('identify uri is passed on to core', () async {
      const uri = 'https://example.org';
      await typedWalletCore.identifyUri(uri);
      verify(core.identifyUri(uri: uri)).called(1);
    });
  });

  group('cancelPidIssuance', () {
    test('cancel pid issuance is passed on to core', () async {
      await typedWalletCore.cancelPidIssuance();
      verify(core.cancelPidIssuance()).called(1);
    });
  });

  group('observeConfig', () {
    test('configuration is fetched through core by setting the configuration stream', () async {
      when(core.setConfigurationStream()).thenAnswer(
        (_) => Stream.value(const FlutterConfiguration(inactiveLockTimeout: 0, backgroundLockTimeout: 0)),
      );
      // Verify we don't observe the stream pre-emptively
      verifyNever(core.setConfigurationStream());
      // But make sure we do call into the core once we check the configuration stream
      await typedWalletCore.observeConfig().first;
      verify(core.setConfigurationStream()).called(1);
    });
  });

  group('acceptOfferedPid', () {
    test('accept offered pid is passed on to core', () async {
      await typedWalletCore.acceptOfferedPid(samplePin);
      verify(core.acceptPidIssuance(pin: samplePin)).called(1);
    });
  });

  group('rejectOfferedPid', () {
    test('reject offered pid is passed on to core', () async {
      await typedWalletCore.rejectOfferedPid();
      verify(core.rejectPidIssuance()).called(1);
    });
  });

  group('resetWallet', () {
    test('reset wallet pid is passed on to core', () async {
      await typedWalletCore.resetWallet();
      verify(core.resetWallet()).called(1);
    });
  });

  group('observeCards', () {
    test('observeCards should fetch cards through WalletCore', () {
      List<Card> mockCards = [
        const Card(persistence: CardPersistence.stored(id: '0'), docType: 'pid_id', attributes: []),
        const Card(persistence: CardPersistence.stored(id: '0'), docType: 'pid_address', attributes: []),
      ];
      when(core.setCardsStream()).thenAnswer((realInvocation) => Stream.value(mockCards));
      expect(
        typedWalletCore.observeCards(),
        emitsInOrder([hasLength(mockCards.length)]),
      );
    });

    test('observeCards should emit a new value when WalletCore exposes new cards', () {
      List<Card> initialCards = [
        const Card(persistence: CardPersistence.stored(id: '0'), docType: 'pid_id', attributes: [])
      ];
      List<Card> updatedCards = [
        const Card(persistence: CardPersistence.stored(id: '0'), docType: 'pid_id', attributes: []),
        const Card(persistence: CardPersistence.stored(id: '0'), docType: 'pid_address', attributes: []),
      ];
      when(core.setCardsStream()).thenAnswer((realInvocation) => Stream.fromIterable([[], initialCards, updatedCards]));

      expect(
        typedWalletCore.observeCards(),
        emitsInOrder([hasLength(0), hasLength(initialCards.length), hasLength(updatedCards.length)]),
      );
    });

    test('observeCards should emit only the last value on a new subscription', () async {
      List<Card> initialCards = [
        const Card(persistence: CardPersistence.stored(id: '0'), docType: 'pid_id', attributes: [])
      ];
      List<Card> updatedCards = [
        const Card(persistence: CardPersistence.stored(id: '0'), docType: 'pid_id', attributes: []),
        const Card(persistence: CardPersistence.stored(id: '0'), docType: 'pid_address', attributes: []),
      ];
      when(core.setCardsStream()).thenAnswer((realInvocation) => Stream.fromIterable([initialCards, updatedCards]));

      /// This makes sure the observeCards() had a chance initialize
      await typedWalletCore.observeCards().take(2).last;

      /// On a new subscription we now only expect to see the last value
      expect(typedWalletCore.observeCards(), emitsInOrder([hasLength(updatedCards.length)]));
    });
  });

  ///Verify that methods convert potential [FfiException]s into the expected [CoreError]s
  group('handleCoreException', () {
    /// Create a [FfiException] that should be converted to a [CoreError]
    final flutterApiError = FlutterApiError(type: FlutterApiErrorType.generic, description: null, data: null);
    final ffiException = FfiException('RESULT_ERROR', jsonEncode(flutterApiError));

    test('isValidPin', () async {
      when(core.isValidPin(pin: samplePin)).thenAnswer((_) async => throw ffiException);
      expect(() async => await typedWalletCore.isValidPin(samplePin), throwsA(isA<CoreError>()));
    });

    test('register', () async {
      when(core.register(pin: samplePin)).thenAnswer((_) async => throw ffiException);
      expect(() async => await typedWalletCore.register(samplePin), throwsA(isA<CoreError>()));
    });

    test('isRegistered', () async {
      when(core.hasRegistration()).thenAnswer((_) async => throw ffiException);
      expect(() async => await typedWalletCore.isRegistered(), throwsA(isA<CoreError>()));
    });

    test('lockWallet', () async {
      when(core.lockWallet()).thenAnswer((_) async => throw ffiException);
      expect(() async => await typedWalletCore.lockWallet(), throwsA(isA<CoreError>()));
    });

    test('unlockWallet', () async {
      when(core.unlockWallet(pin: samplePin)).thenAnswer((_) async => throw ffiException);
      expect(() async => await typedWalletCore.unlockWallet(samplePin), throwsA(isA<CoreError>()));
    });

    test('createPidIssuanceRedirectUri', () async {
      when(core.createPidIssuanceRedirectUri()).thenAnswer((_) async => throw ffiException);
      expect(() async => await typedWalletCore.createPidIssuanceRedirectUri(), throwsA(isA<CoreError>()));
    });

    test('identifyUri', () async {
      when(core.identifyUri(uri: 'https://example.org')).thenThrow(ffiException);
      expect(() => typedWalletCore.identifyUri('https://example.org'), throwsA(isA<CoreError>()));
    });

    test('cancelPidIssuance', () async {
      when(core.cancelPidIssuance()).thenAnswer((_) async => throw ffiException);
      expect(() async => await typedWalletCore.cancelPidIssuance(), throwsA(isA<CoreError>()));
    });

    test('acceptOfferedPid', () async {
      when(core.acceptPidIssuance(pin: samplePin)).thenAnswer((_) async => throw ffiException);
      expect(() async => await typedWalletCore.acceptOfferedPid(samplePin), throwsA(isA<CoreError>()));
    });

    test('rejectOfferedPid', () async {
      when(core.rejectPidIssuance()).thenAnswer((_) async => throw ffiException);
      expect(() async => await typedWalletCore.rejectOfferedPid(), throwsA(isA<CoreError>()));
    });

    test('resetWallet', () async {
      when(core.resetWallet()).thenAnswer((_) async => throw ffiException);
      expect(() async => await typedWalletCore.resetWallet(), throwsA(isA<CoreError>()));
    });
  });
}
