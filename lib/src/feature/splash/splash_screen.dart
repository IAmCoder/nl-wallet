import 'package:fimber/fimber.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import '../../wallet_routes.dart';
import 'bloc/splash_bloc.dart';

class SplashScreen extends StatelessWidget {
  const SplashScreen({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return BlocListener<SplashBloc, SplashState>(
      listenWhen: (prev, current) => current is SplashLoaded,
      listener: (context, state) {
        if (state is SplashLoaded) {
          if (state.isInitialized) {
            Navigator.pushReplacementNamed(context, WalletRoutes.pinRoute);
          } else {
            Fimber.d("Not initialized, prefer something like Fimber for logging?");
          }
        }
      },
      child: Scaffold(
        body: Center(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.center,
            mainAxisSize: MainAxisSize.min,
            children: const [
              FlutterLogo(size: 80),
              SizedBox(height: 16),
              Text("EDI Wallet"),
              SizedBox(height: 16),
              CircularProgressIndicator(),
            ],
          ),
        ),
      ),
    );
  }
}
