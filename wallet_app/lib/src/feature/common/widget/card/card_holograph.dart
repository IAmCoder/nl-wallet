import 'package:flutter/material.dart';

import '../svg_or_image.dart';
import 'gyro_gradient_overlay.dart';

final Color _kWhite0 = Colors.white.withOpacity(0);
final Color _kWhite5 = Colors.white.withOpacity(0.05);

class CardHolograph extends StatelessWidget {
  /// Reference to the asset that should be rendered as the holograph
  final String holograph;

  /// Brightness of the holograph, when set to [Brightness.light] the
  /// rendered holograph will be brighter.
  final Brightness brightness;

  const CardHolograph({
    required this.holograph,
    required this.brightness,
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    return Stack(
      alignment: Alignment.centerRight,
      children: [
        GyroGradientOverlay(
          gradient: _generateHoloGradient(),
          child: SvgOrImage(
            asset: holograph,
            fit: BoxFit.scaleDown,
            alignment: Alignment.center,
          ),
        ),
        GyroGradientOverlay(
          gradient: _generateOutlineGradient(),
          child: Container(
            width: 132,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              border: Border.all(
                color: Colors.white,
                width: 1,
              ),
            ),
          ),
        ),
      ],
    );
  }

  Gradient _generateHoloGradient() => LinearGradient(
        colors: [
          _kWhite5,
          Colors.white.withOpacity(brightness == Brightness.light ? .6 : .21),
          _kWhite5,
        ],
        begin: Alignment.topCenter,
        end: Alignment.bottomCenter,
      );

  Gradient _generateOutlineGradient() => LinearGradient(
        colors: [
          _kWhite0,
          Colors.white.withOpacity(brightness == Brightness.light ? .6 : .21),
          _kWhite0,
        ],
        begin: Alignment.topCenter,
        end: Alignment.bottomCenter,
      );
}
