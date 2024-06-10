import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_gen/gen_l10n/app_localizations.dart';

import '../../domain/model/bloc/error_state.dart';
import '../../util/cast_util.dart';
import '../../util/extension/build_context_extension.dart';
import '../../wallet_constants.dart';
import '../common/widget/button/button_content.dart';
import '../common/widget/button/list_button.dart';
import '../common/widget/pin_header.dart';
import '../common/widget/wallet_scrollbar.dart';
import '../error/error_screen.dart';
import '../forgot_pin/forgot_pin_screen.dart';
import '../pin_blocked/pin_blocked_screen.dart';
import '../pin_timeout/pin_timeout_screen.dart';
import 'bloc/pin_bloc.dart';
import 'widget/pin_field.dart';
import 'widget/pin_keyboard.dart';

/// If the user has less then [kNonFinalRoundMLeftoverAttemptsMentionThreshold] attempts left
/// to enter the correct pin, we switch to showing the counter inside the warning dialog.
const kNonFinalRoundMLeftoverAttemptsMentionThreshold = 3;

/// Signature for a function that creates a widget while providing the leftover pin attempts.
/// [attempts] being null indicates that this is the first attempt.
/// [isFinalAttempt] being true indicates it's the final attempt (followed by the user being blocked, i.e. no more timeout)
typedef PinHeaderBuilder = Widget Function(
  BuildContext context,
  int? attemptsLeftInRound,
  bool isFinalRound,
);

/// Signature for a function that is called on any state change exposed by the [PinBloc]. When this method
/// is provided AND returns true for the given [PinState], the state is considered consumed and will not be handled
/// by the [PinPage] to trigger potential (navigation) events.
typedef PinStateInterceptor = bool Function(BuildContext context, PinState state);

/// Signature for a function that is called when the [PinBloc] exposes an [ErrorState]
typedef OnPinErrorCallback = void Function(BuildContext context, ErrorState state);

/// Signature for a function that is called when the user has entered the correct pin.
/// [returnUrl] is the url that the user should be redirected to (if not null).
typedef OnPinValidatedCallback = void Function(String? returnUrl);

/// Provides pin validation and renders any errors based on the state from the nearest [PinBloc].
class PinPage extends StatelessWidget {
  /// Called when pin entry was successful
  final OnPinValidatedCallback onPinValidated;

  /// Called for every state change exposed by the [PinBloc]. When [onStateChanged] is
  /// provided and it returns true, the event is not processed by this [PinPage].
  final PinStateInterceptor? onStateChanged;

  /// Called when the [PinBloc] exposes an [ErrorState]. When [onPinError] is provided
  /// the [ErrorState]s are no longer handled by this [PinPage].
  final OnPinErrorCallback? onPinError;

  /// Build a custom header, when null it defaults to [_defaultHeaderBuilder].
  final PinHeaderBuilder? headerBuilder;

  /// Draw a divider at the top of the screen when in landscape mode
  final bool showTopDivider;

  /// The color used to draw the keyboard keys & pin dots
  final Color? keyboardColor;

  const PinPage({
    required this.onPinValidated,
    this.onStateChanged,
    this.onPinError,
    this.headerBuilder,
    this.keyboardColor,
    this.showTopDivider = false,
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    return BlocListener<PinBloc, PinState>(
      listener: (context, state) async {
        final l10n = context.l10n;
        if (state is PinEntryInProgress) {
          Future.delayed(kDefaultAnnouncementDelay).then((value) {
            if (state.afterBackspacePressed) {
              _announceEnteredDigits(l10n, state.enteredDigits);
            } else if (state.enteredDigits > 0 && state.enteredDigits < kPinDigits) {
              _announceEnteredDigits(l10n, state.enteredDigits);
            }
          });
        }

        /// Check for state interceptions
        if (onStateChanged?.call(context, state) == true) return;
        if (onPinError != null && state is ErrorState) {
          onPinError!(context, state as ErrorState);
          return;
        }

        /// Process the state change
        switch (state) {
          case PinValidateSuccess():
            onPinValidated.call(state.returnUrl);
            break;
          case PinValidateTimeout():
            PinTimeoutScreen.show(context, state.expiryTime);
            break;
          case PinValidateBlocked():
            PinBlockedScreen.show(context);
            break;
          case PinValidateNetworkError():
            ErrorScreen.showNetwork(context, secured: false, networkError: tryCast(state));
            break;
          case PinValidateGenericError():
            ErrorScreen.showGeneric(context, secured: false);
            break;
          case PinValidateFailure():
            _showErrorDialog(context, state);
            break;

          /// No need to handle these explicitly as events for now.
          case PinEntryInProgress():
          case PinValidateInProgress():
            break;
        }
      },
      child: OrientationBuilder(
        builder: (context, orientation) {
          switch (orientation) {
            case Orientation.portrait:
              return _buildPortrait(context);
            case Orientation.landscape:
              return _buildLandscape(context);
          }
        },
      ),
    );
  }

  Widget _buildPortrait(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        Expanded(child: _buildHeader(headerBuilder ?? _defaultHeaderBuilder)),
        _buildPinField(),
        const SizedBox(height: 18),
        _buildPinKeyboard(),
        SafeArea(
          child: _buildForgotCodeButton(context),
        ),
      ],
    );
  }

  Widget _buildLandscape(BuildContext context) {
    final leftSection = Expanded(
      child: Column(
        children: [
          Expanded(
            child: SafeArea(
              right: false,
              top: false,
              bottom: false,
              child: _buildHeader(headerBuilder ?? _defaultHeaderBuilder),
            ),
          ),
          SafeArea(
            top: false,
            right: false,
            child: _buildForgotCodeButton(context),
          ),
        ],
      ),
    );
    final rightSection = Expanded(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 24.0, horizontal: 16),
        child: SafeArea(
          left: false,
          right: true,
          top: false,
          bottom: false,
          child: Column(
            children: [
              _buildPinField(),
              const SizedBox(height: 16),
              Expanded(
                child: _buildPinKeyboard(),
              ),
            ],
          ),
        ),
      ),
    );
    return Column(
      mainAxisSize: MainAxisSize.max,
      children: [
        showTopDivider ? const Divider(height: 1) : const SizedBox.shrink(),
        Expanded(
          child: Row(
            children: [
              leftSection,
              const VerticalDivider(width: 1),
              rightSection,
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildHeader(PinHeaderBuilder builder) {
    return WalletScrollbar(
      child: CustomScrollView(
        slivers: [
          SliverFillRemaining(
            hasScrollBody: false,
            child: BlocBuilder<PinBloc, PinState>(
              builder: (context, state) {
                if (state is PinValidateFailure) {
                  return builder(context, state.attemptsLeftInRound, state.isFinalRound);
                } else {
                  return builder(context, null, false);
                }
              },
            ),
          ),
        ],
      ),
    );
  }

  /// Builds the default pin header, as shown on the 'unlock the app' screen.
  Widget _defaultHeaderBuilder(BuildContext context, int? attemptsLeftInRound, bool isFinalRound) {
    return PinHeader(
      title: context.l10n.pinScreenHeader,
      contentAlignment: context.isLandscape ? Alignment.centerLeft : Alignment.topCenter,
      textAlign: context.isLandscape ? TextAlign.start : TextAlign.center,
    );
  }

  Widget _buildPinField() {
    return BlocBuilder<PinBloc, PinState>(
      builder: (context, state) {
        return PinField(
          color: keyboardColor,
          digits: kPinDigits,
          enteredDigits: _resolveEnteredDigits(state),
          state: _resolvePinFieldState(state),
        );
      },
    );
  }

  Widget _buildForgotCodeButton(BuildContext context) {
    return ListButton(
      mainAxisAlignment: context.isLandscape ? MainAxisAlignment.start : MainAxisAlignment.center,
      icon: const Icon(Icons.help_outline_rounded),
      onPressed: () => ForgotPinScreen.show(context),
      iconPosition: IconPosition.start,
      text: Text(context.l10n.pinScreenForgotPinCta),
      dividerSide: DividerSide.top,
    );
  }

  Widget _buildPinKeyboard() {
    return BlocBuilder<PinBloc, PinState>(
      builder: (context, state) {
        return AnimatedOpacity(
          duration: kDefaultAnimationDuration,
          opacity: state is PinValidateInProgress ? 0.3 : 1,
          child: PinKeyboard(
            color: keyboardColor,
            onKeyPressed: _digitKeysEnabled(state) ? (digit) => context.bloc.add(PinDigitPressed(digit)) : null,
            onBackspacePressed:
                _backspaceKeyEnabled(state) ? () => context.bloc.add(const PinBackspacePressed()) : null,
            onBackspaceLongPressed:
                _backspaceKeyEnabled(state) ? () => context.bloc.add(const PinClearPressed()) : null,
          ),
        );
      },
    );
  }

  bool _digitKeysEnabled(PinState state) {
    return switch (state) {
      PinEntryInProgress() => true,
      PinValidateFailure() => true,
      PinValidateTimeout() => true,
      PinValidateNetworkError() => true,
      PinValidateGenericError() => true,
      PinValidateInProgress() => false,
      PinValidateSuccess() => false,
      PinValidateBlocked() => false,
    };
  }

  bool _backspaceKeyEnabled(PinState state) {
    return switch (state) {
      PinEntryInProgress() => true,
      PinValidateFailure() => true,
      PinValidateInProgress() => false,
      PinValidateSuccess() => false,
      PinValidateTimeout() => false,
      PinValidateBlocked() => false,
      PinValidateNetworkError() => false,
      PinValidateGenericError() => false,
    };
  }

  int _resolveEnteredDigits(PinState state) {
    return switch (state) {
      PinEntryInProgress() => state.enteredDigits,
      PinValidateInProgress() => kPinDigits,
      PinValidateSuccess() => kPinDigits,
      PinValidateFailure() => 0,
      PinValidateTimeout() => 0,
      PinValidateBlocked() => 0,
      PinValidateNetworkError() => 0,
      PinValidateGenericError() => 0,
    };
  }

  PinFieldState _resolvePinFieldState(PinState state) {
    if (state is PinValidateInProgress) return PinFieldState.loading;
    if (state is PinValidateFailure) return PinFieldState.error;
    return PinFieldState.idle;
  }

  void _announceEnteredDigits(AppLocalizations l10n, int enteredDigits) {
    SemanticsService.announce(
      l10n.pinEnteredDigitsAnnouncement(kPinDigits - enteredDigits),
      TextDirection.ltr,
    );
  }

  Future<void> _showErrorDialog(BuildContext context, PinValidateFailure reason) async {
    final body = _pinErrorDialogBody(context, reason);
    return showPinErrorDialog(context, body);
  }

  static Future<void> showPinErrorDialog(BuildContext context, String description) async {
    final title = context.l10n.pinErrorDialogTitle;
    return showDialog<void>(
      context: context,
      barrierDismissible: false,
      builder: (BuildContext context) {
        return AlertDialog(
          scrollable: true,
          semanticLabel: title,
          title: Text(title, style: context.textTheme.displayMedium),
          content: Text(description, style: context.textTheme.bodyLarge),
          actions: <Widget>[
            TextButton(
              child: Text(context.l10n.pinErrorDialogForgotCodeCta.toUpperCase()),
              onPressed: () {
                Navigator.of(context).pop();
                ForgotPinScreen.show(context);
              },
            ),
            TextButton(
              child: Text(context.l10n.pinErrorDialogCloseCta.toUpperCase()),
              onPressed: () => Navigator.of(context).pop(),
            ),
          ],
        );
      },
    );
  }

  String _pinErrorDialogBody(BuildContext context, PinValidateFailure reason) {
    if (reason.isFinalRound) {
      // Final round is a special case where the user has X attempts left before the app is blocked.
      if (reason.attemptsLeftInRound > 1) {
        return context.l10n.pinErrorDialogFinalRoundNonFinalAttempt(reason.attemptsLeftInRound);
      } else {
        return context.l10n.pinErrorDialogFinalRoundFinalAttempt;
      }
    } else {
      // Regular case where the user has X attempts left before the app is temporary blocked.
      switch (reason.attemptsLeftInRound) {
        case 1:
          return context.l10n.pinErrorDialogNonFinalRoundFinalAttempt;
        case < kNonFinalRoundMLeftoverAttemptsMentionThreshold:
          return context.l10n.pinErrorDialogNonFinalRoundNonFinalAttempt(reason.attemptsLeftInRound);
        default:
          return context.l10n.pinErrorDialogNonFinalRoundInitialAttempt;
      }
    }
  }
}

extension _PinPageExtensions on BuildContext {
  PinBloc get bloc => read<PinBloc>();
}
