import 'package:flutter/material.dart';

import '../../../util/extension/build_context_extension.dart';

class KeyboardDigitKey extends StatelessWidget {
  final int digit;
  final Function(int)? onKeyPressed;

  const KeyboardDigitKey({required this.digit, this.onKeyPressed, super.key});

  @override
  Widget build(BuildContext context) {
    return Expanded(
      child: InkWell(
        onTap: onKeyPressed == null ? null : () => onKeyPressed!(digit),
        child: Center(
          child: Semantics(
            keyboardKey: true,
            button: true,
            onTapHint: context.l10n.pinKeyboardWCAGDigitKeyTapHint,
            child: Text(
              digit.toString(),
              textAlign: TextAlign.center,
            ),
          ),
        ),
      ),
    );
  }
}
