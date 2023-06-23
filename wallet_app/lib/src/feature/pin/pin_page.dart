import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import '../../util/extension/build_context_extension.dart';
import '../../wallet_constants.dart';
import '../common/widget/button/text_icon_button.dart';
import '../common/widget/wallet_logo.dart';
import '../error/error_screen.dart';
import '../forgot_pin/forgot_pin_screen.dart';
import '../pin_blocked/pin_blocked_screen.dart';
import '../pin_timeout/pin_timeout_screen.dart';
import 'bloc/pin_bloc.dart';
import 'widget/pin_field.dart';
import 'widget/pin_keyboard.dart';

/// Signature for a function that creates a widget while providing the leftover pin attempts.
/// [attempts] being null indicates that this is the first attempt.
/// [isFinalAttempt] being true indicates it's the final attempt (followed by the user being blocked, i.e. no more timeout)
typedef PinHeaderBuilder = Widget Function(BuildContext context, int? attempts, bool isFinalAttempt);

/// Provides pin validation and renders any errors based on the state from the nearest [PinBloc].
class PinPage extends StatelessWidget {
  final VoidCallback? onPinValidated;
  final PinHeaderBuilder? headerBuilder;

  const PinPage({
    this.onPinValidated,
    this.headerBuilder,
    Key? key,
  }) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return BlocListener<PinBloc, PinState>(
      listener: (context, state) {
        if (state is PinValidateSuccess) onPinValidated?.call();
        if (state is PinValidateServerError) {
          ErrorScreen.showGeneric(context, secured: false);
        }
        if (state is PinValidateTimeout) {
          PinTimeoutScreen.show(context, state.expiryTime);
        }
        if (state is PinValidateBlocked) {
          PinBlockedScreen.show(context);
        }
      },
      child: OrientationBuilder(
        builder: (context, orientation) {
          switch (orientation) {
            case Orientation.portrait:
              return _buildPortrait();
            case Orientation.landscape:
              return _buildLandscape();
          }
        },
      ),
    );
  }

  Widget _buildPortrait() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        if (headerBuilder == null) const Spacer(),
        _buildHeader(headerBuilder ?? _defaultHeaderBuilder),
        const Spacer(),
        _buildPinField(),
        const SizedBox(height: 18),
        _buildForgotCodeButton(),
        const Spacer(),
        _buildPinKeyboard(),
      ],
    );
  }

  Widget _buildLandscape() {
    return Row(
      children: [
        Expanded(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              _buildHeader(headerBuilder ?? _buildTextHeader),
              const SizedBox(height: 24),
              _buildPinField(),
              const SizedBox(height: 18),
              _buildForgotCodeButton(),
            ],
          ),
        ),
        Expanded(
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 16),
            child: _buildPinKeyboard(),
          ),
        ),
      ],
    );
  }

  Widget _buildHeader(PinHeaderBuilder builder) {
    return BlocBuilder<PinBloc, PinState>(
      builder: (context, state) {
        if (state is PinValidateFailure) {
          return builder(context, state.leftoverAttempts, state.isFinalAttempt);
        } else {
          return builder(context, null, false);
        }
      },
    );
  }

  Widget _defaultHeaderBuilder(BuildContext context, int? attempts, bool isFinalAttempt) {
    return Column(
      children: [
        const WalletLogo(size: 80),
        const SizedBox(height: 24),
        _buildTextHeader(context, attempts, isFinalAttempt),
      ],
    );
  }

  Widget _buildTextHeader(BuildContext context, int? attempts, bool isFinalAttempt) {
    if (attempts == null) {
      return Column(
        children: [
          Text(
            context.l10n.pinScreenHeader,
            style: context.textTheme.displaySmall,
            textAlign: TextAlign.center,
          ),
          Text(
            '' /* makes sure the UI doesn't jump around */,
            style: context.textTheme.bodyLarge,
          ),
        ],
      );
    } else {
      return Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            context.l10n.pinScreenErrorHeader,
            style: context.textTheme.displaySmall?.copyWith(color: context.colorScheme.error),
            textAlign: TextAlign.center,
          ),
          Text(
            context.l10n.pinScreenAttemptsCount(attempts),
            style: context.textTheme.bodyLarge?.copyWith(color: context.colorScheme.error),
            textAlign: TextAlign.center,
          ),
        ],
      );
    }
  }

  Widget _buildPinField() {
    return BlocBuilder<PinBloc, PinState>(
      builder: (context, state) {
        return PinField(
          digits: kPinDigits,
          enteredDigits: _resolveEnteredDigits(state),
        );
      },
    );
  }

  Widget _buildForgotCodeButton() {
    return BlocBuilder<PinBloc, PinState>(
      builder: (context, state) {
        final buttonEnabled = state is PinEntryInProgress || state is PinValidateFailure;
        return TextIconButton(
          onPressed: buttonEnabled ? () => ForgotPinScreen.show(context) : null,
          child: Text(context.l10n.pinScreenForgotPinCta),
        );
      },
    );
  }

  Widget _buildPinKeyboard() {
    return BlocBuilder<PinBloc, PinState>(
      builder: (context, state) {
        return AnimatedOpacity(
          duration: kDefaultAnimationDuration,
          opacity: state is PinValidateInProgress ? 0.3 : 1,
          child: PinKeyboard(
            onKeyPressed:
                _digitKeysEnabled(state) ? (digit) => context.read<PinBloc>().add(PinDigitPressed(digit)) : null,
            onBackspacePressed:
                _backspaceKeyEnabled(state) ? () => context.read<PinBloc>().add(const PinBackspacePressed()) : null,
          ),
        );
      },
    );
  }

  bool _digitKeysEnabled(PinState state) {
    if (state is PinValidateServerError) return true;
    if (state is PinValidateTimeout) return true;
    if (state is PinEntryInProgress) return true;
    if (state is PinValidateFailure) return true;
    return false;
  }

  bool _backspaceKeyEnabled(PinState state) {
    if (state is PinEntryInProgress) return true;
    if (state is PinValidateFailure) return true;
    return false;
  }

  int _resolveEnteredDigits(PinState state) {
    if (state is PinEntryInProgress) return state.enteredDigits;
    if (state is PinValidateInProgress) return kPinDigits;
    if (state is PinValidateSuccess) return kPinDigits;
    if (state is PinValidateFailure) return kPinDigits;
    return 0;
  }
}
