import 'dart:async';

import 'package:fimber/fimber.dart';
import 'package:flutter/material.dart';
import 'package:rxdart/rxdart.dart';
import 'package:uni_links/uni_links.dart';

import '../../../bridge_generated.dart';
import '../../domain/model/navigation/navigation_request.dart';
import '../../domain/usecase/deeplink/decode_deeplink_usecase.dart';
import '../../domain/usecase/pid/update_pid_issuance_status_usecase.dart';
import '../../domain/usecase/wallet/is_wallet_initialized_with_pid_usecase.dart';
import '../../domain/usecase/wallet/observe_wallet_lock_usecase.dart';
import '../../domain/usecase/wallet/setup_mocked_wallet_usecase.dart';
import '../../navigation/wallet_routes.dart';
import '../../wallet_core/typed_wallet_core.dart';
import 'app_lifecycle_service.dart';

class DeeplinkService {
  /// Key that holds [NavigatorState], used to perform navigation from a non-Widget.
  final GlobalKey<NavigatorState> _navigatorKey;

  /// The [TypedWalletCore], used as a fallback for handling deeplinks
  final TypedWalletCore _walletCore;

  /// StreamController to which all incoming [Uri]s are published
  final _uriController = StreamController<Uri?>.broadcast();

  /// A queued [NavigationRequest], when navigation can't be performed (e.g. app
  /// not in a state where it can be handled) maximum 1 [NavigationRequest] is queued
  /// here to be handled when [processQueue] is called.
  NavigationRequest? _queuedRequest;

  /// Service used to observe the current [AppLifecycleState], so that [Uri]s are
  /// only processed when the app is in the foreground.
  final AppLifecycleService _appLifecycleService;

  final DecodeDeeplinkUseCase _decodeDeeplinkUseCase;
  final UpdatePidIssuanceStatusUseCase _updatePidIssuanceStatusUseCase;
  final ObserveWalletLockUseCase _observeWalletLockUseCase;
  final IsWalletInitializedWithPidUseCase _isWalletInitializedWithPidUseCase;
  final SetupMockedWalletUseCase _setupMockedWalletUseCase;

  DeeplinkService(
    this._navigatorKey,
    this._decodeDeeplinkUseCase,
    this._updatePidIssuanceStatusUseCase,
    this._isWalletInitializedWithPidUseCase,
    this._setupMockedWalletUseCase,
    this._observeWalletLockUseCase,
    this._walletCore,
    this._appLifecycleService,
  ) {
    // Delay the actual processing of the (last seen) [Uri] until the app is resumed and unlocked
    // Note: The order and delay is important, as the apps 'locked' flag is set when the [AppLifecycleState]
    //       changes. Meaning that without that debounce the isLockedStream could produce a stale value.
    _uriController.stream
        .whereNotNull()
        .debounce((uri) => _appLifecycleService.observe().where((state) => state == AppLifecycleState.resumed))
        .debounceTime(const Duration(milliseconds: 200))
        .debounce((uri) => _debounceUriHost(uri.host))
        .listen(processUri);

    // Pass the [Uri]s to the [_uriController] so they can be processed when the app is unlocked
    getInitialUri().then((uri) => _uriController.add(uri));
    uriLinkStream.listen((uri) => _uriController.add(uri));
  }

  /// Determines debouncing based on [Uri] host and wallet lock state:
  /// - Deep dive links are always allowed, no debounce
  /// - Non-deep dive links are only allowed when the wallet is unlocked, debounce
  Stream<bool> _debounceUriHost(String host) {
    return host == _decodeDeeplinkUseCase.deepDiveHost
        ? Stream.value(true)
        : _observeWalletLockUseCase.invoke().where((locked) => !locked);
  }

  /// Process the incoming [Uri], first attempting to resolve it inside the wallet_app, but if the link is
  /// unsupported, it is passed on to the wallet_core to handle it there.
  @visibleForTesting
  Future<void> processUri(Uri uri) async {
    Fimber.d('Processing uri: $uri');
    final navRequest = _decodeDeeplinkUseCase.invoke(uri);
    if (navRequest != null) {
      await _handleNavRequest(navRequest);
    } else {
      await _delegateToWalletCore(uri);
    }
  }

  /// Pass the [Uri] to the wallet_core to let it decide how to process it, handling the result.
  Future<void> _delegateToWalletCore(Uri uri) async {
    _walletCore.processUri(uri).listen((event) {
      Fimber.d('wallet_core processUri response: $event');
      event.when(
        pidIssuance: (PidIssuanceEvent state) {
          // We only pass on the [PidIssuanceEvent] here (no navigation) since:
          // - if the app did not cold start the user is already in the correct place
          // - else if the wallet is not yet registered, PidIssuance not yet appropriate and it will be re-initiated later.
          // - else if the wallet is registered but the PID is not yet retrieved, the user will end up in the personalize flow,
          //   the correct state will be rendered because we notify the repository that authentication is in process.
          // - else if the wallet is registered and the PID is available, PidIssuance is no longer relevant.
          _updatePidIssuanceStatusUseCase.invoke(state);
        },
        unknownUri: () => Fimber.d('walletCore did not recognize $uri, ignoring.'),
      );
    }, onError: (ex) {
      Fimber.e('processUri() threw an exception while processing $uri', ex: ex);
    }, onDone: () {
      Fimber.d('processUri() stream completed');
    });
  }

  /// Process the provided [NavigationRequest], or queue it if the app is in a state where it can't be handled.
  /// Overrides any previously set [NavigationRequest] if this request has to be queued as well.
  Future<void> _handleNavRequest(NavigationRequest request) async {
    if (await _canNavigate(request)) {
      _navigate(request);
    } else {
      Fimber.d('Not yet ready to handle $request, queued and awaiting call to DeeplinkService.processQueue().');
      _queuedRequest = request;
    }
  }

  /// Check whether the apps current state allows navigation based on the provided [NavigationRequest]
  /// If no [NavigationRequest.navigatePrerequisite] is provided, the wallet is checked to be initialized with a PID.
  Future<bool> _canNavigate(NavigationRequest request) {
    return request.navigatePrerequisite == null ? _isWalletInitializedWithPidUseCase.invoke() : Future.value(true);
  }

  Future<void> _navigate(NavigationRequest request) async {
    _handleNavigatePrerequisite(request);

    _navigatorKey.currentState?.restorablePushNamedAndRemoveUntil(
      request.destination,
      ModalRoute.withName(WalletRoutes.homeRoute),
      arguments: request.argument,
    );
  }

  Future<void> _handleNavigatePrerequisite(NavigationRequest request) async {
    switch (request.navigatePrerequisite) {
      case NavigationPrerequisite.setupMockedWallet:
        await _setupMockedWalletUseCase.invoke();
        break;
      case null:
        return;
    }
  }

  /// Process any outstanding [NavigationRequest] and consume it if it can be handled.
  Future<void> processQueue() async {
    final queuedRequest = _queuedRequest;
    if (queuedRequest != null && await _canNavigate(queuedRequest)) {
      _queuedRequest = null;
      _navigate(queuedRequest);
    }
  }
}
