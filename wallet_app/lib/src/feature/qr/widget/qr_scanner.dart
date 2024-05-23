import 'package:fimber/fimber.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

import '../../../util/extension/build_context_extension.dart';
import '../../common/widget/centered_loading_indicator.dart';
import '../bloc/qr_bloc.dart';

class QrScanner extends StatefulWidget {
  const QrScanner({super.key});

  @override
  State<QrScanner> createState() => _QrScannerState();
}

class _QrScannerState extends State<QrScanner> {
  final MobileScannerController cameraController = MobileScannerController(formats: [BarcodeFormat.qrCode]);

  @override
  Widget build(BuildContext context) {
    return MobileScanner(
      controller: cameraController,
      overlayBuilder: _buildOverlay,
      placeholderBuilder: (context, child) => const CenteredLoadingIndicator(),
      errorBuilder: (context, ex, child) {
        Fimber.e('Failed to start camera', ex: ex);
        return Center(child: Text(context.l10n.errorScreenGenericHeadline));
      },
      onDetect: (capture) {
        final event = QrScanCodeDetected(capture.barcodes.first);
        context.read<QrBloc>().add(event);
      },
    );
  }

  Widget _buildOverlay(BuildContext context, BoxConstraints constraints) {
    return Stack(
      children: [
        _buildAlignedScanQrHint(),
        _buildPositionedFlashLightButton(),
      ],
    );
  }

  Widget _buildAlignedScanQrHint() {
    return Align(
      alignment: Alignment.topCenter,
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.symmetric(vertical: 8),
        margin: EdgeInsets.only(top: context.mediaQuery.padding.top),
        color: context.theme.appBarTheme.backgroundColor?.withOpacity(0.9),
        child: Text(
          context.l10n.qrScreenScanHint,
          textAlign: TextAlign.center,
          style: context.textTheme.bodyLarge,
        ),
      ),
    );
  }

  Widget _buildPositionedFlashLightButton() {
    if (cameraController.value.torchState == TorchState.unavailable) return const SizedBox.shrink();
    bool isOn = cameraController.value.torchState.isOn;
    final buttonRadius = BorderRadius.circular(200);
    return Positioned(
      bottom: 32,
      left: 0,
      right: 0,
      child: Center(
        child: Material(
          color: Colors.white,
          borderRadius: buttonRadius,
          child: Semantics(
            button: true,
            child: InkWell(
              borderRadius: buttonRadius,
              onTap: () => _toggleFlashLight(context),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 16),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      isOn ? Icons.flashlight_on_outlined : Icons.flashlight_off_outlined,
                      color: context.colorScheme.onSecondary,
                      size: 16,
                      semanticLabel: isOn ? context.l10n.generalOn : context.l10n.generalOff,
                    ),
                    const SizedBox(width: 12),
                    Text(
                      isOn ? context.l10n.qrScreenDisableTorchCta : context.l10n.qrScreenEnableTorchCta,
                      style: context.textTheme.labelLarge?.copyWith(color: context.colorScheme.onSecondary),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  void _toggleFlashLight(BuildContext context) {
    final l10n = context.l10n;
    final currentOnState = cameraController.value.torchState.isOn;
    final postToggleOnState = !currentOnState;
    cameraController.toggleTorch().then((value) async {
      if (postToggleOnState) {
        SemanticsService.announce(l10n.flashlightEnabledWCAGAnnouncement, TextDirection.ltr);
      } else {
        SemanticsService.announce(l10n.flashlightDisabledWCAGAnnouncement, TextDirection.ltr);
      }
    });
  }
}

extension _TorchStateExtension on TorchState {
  bool get isOn => this == TorchState.on;
}
