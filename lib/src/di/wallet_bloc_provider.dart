import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import '../localization/preferred_locale_cubit.dart';

/// This widget provides the BLoCs and Cubits that should be
/// available app-wide.
class WalletBlocProvider extends StatelessWidget {
  final Widget child;

  const WalletBlocProvider({required this.child, Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return MultiBlocProvider(
      providers: [
        BlocProvider<PreferredLocaleCubit>(
          create: (context) => PreferredLocaleCubit(context.read()),
        ),
      ],
      child: child,
    );
  }
}
